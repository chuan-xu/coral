use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use coral_macro::trace_error;
use coral_runtime::tokio;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use rustls::pki_types;
use rustls::ClientConfig;

use crate::error::CoralRes;

#[allow(dead_code)]
pub struct TcpClient {
    remote_addr: std::net::SocketAddr,
    remote_domain: String,
    tls_cfg: ClientConfig,
}

type TlsSocket = tokio_rustls::client::TlsStream<tokio::net::TcpStream>;

#[allow(dead_code)]
pub(crate) async fn establish_tls_connection(
    addr: &std::net::SocketAddr,
    domain: String,
    tls_conf: Arc<ClientConfig>,
) -> CoralRes<TlsSocket> {
    let tcp_stream = tokio::net::TcpStream::connect(addr).await?;
    let connector = tokio_rustls::TlsConnector::from(tls_conf);
    let domain = pki_types::ServerName::try_from(domain)?;
    let socket = connector.connect(domain, tcp_stream).await?;
    Ok(socket)
}

/// http1.1
pub struct H1<B> {
    inner: hyper::client::conn::http1::SendRequest<B>,
    count: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
}

#[async_trait::async_trait]
impl<R> crate::client::Request<R, hyper::body::Incoming> for H1<R>
where
    R: hyper::body::Body + Send + 'static,
    R::Data: Send,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    async fn send(
        &mut self,
        req: hyper::Request<R>,
    ) -> CoralRes<hyper::Response<hyper::body::Incoming>> {
        let rsp = self.inner.send_request(req).await?;
        Ok(rsp)
    }
}

impl<B> H1<B>
where
    B: hyper::body::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    pub async fn new(
        socket: TlsSocket,
        builder: hyper::client::conn::http1::Builder,
    ) -> CoralRes<Self> {
        let (sender, conn) = builder
            .handshake::<TokioIo<TlsSocket>, B>(TokioIo::new(socket))
            .await?;
        tokio::spawn(async move {
            if let Err(err) = conn.await {
                error!(e = format!("{:?}", err); "http1 client disconnect");
            }
        });
        Ok(Self {
            inner: sender,
            count: Arc::new(AtomicU32::default()),
            state: Arc::new(AtomicU8::default()),
        })
    }
}
impl<B> crate::client::Statistics for H1<B> {
    fn usage_count(&self) -> (u32, u8) {
        (
            self.count.load(Ordering::Acquire),
            self.state.load(Ordering::Acquire),
        )
    }

    fn usage_add(&self) -> crate::client::StatisticsGuard {
        self.count.fetch_add(1, Ordering::AcqRel);
        crate::client::StatisticsGuard(self.count.clone())
    }
}

/// http2.0
pub struct H2<B> {
    inner: hyper::client::conn::http2::SendRequest<B>,
    count: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
}

impl<B> Clone for H2<B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            count: self.count.clone(),
            state: self.state.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<R> crate::client::Request<R, hyper::body::Incoming> for H2<R>
where
    R: hyper::body::Body + Send + Unpin + 'static,
    R::Data: Send,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    async fn send(
        &mut self,
        req: hyper::Request<R>,
    ) -> CoralRes<hyper::Response<hyper::body::Incoming>> {
        let rsp = self.inner.send_request(req).await?;
        Ok(rsp)
    }
}

impl<B> H2<B>
where
    B: hyper::body::Body + Send + Unpin + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    pub async fn new(
        socket: TlsSocket,
        builder: hyper::client::conn::http2::Builder<TokioExecutor>,
    ) -> CoralRes<Self> {
        let (sender, conn) = builder
            .handshake::<TokioIo<TlsSocket>, B>(TokioIo::new(socket))
            .await?;
        tokio::spawn(async move {
            if let Err(err) = conn.await {
                error!(e = format!("{:?}", err); "http2 client disconnect");
            }
        });
        Ok(Self {
            inner: sender,
            count: Arc::new(AtomicU32::default()),
            state: Arc::new(AtomicU8::default()),
        })
    }
}

impl<B> crate::client::Statistics for H2<B> {
    fn usage_count(&self) -> (u32, u8) {
        let is_closed = self.inner.is_closed();
        match self.count.load(Ordering::Acquire) {
            0 => {
                if is_closed {
                    let mut cur = crate::client::NORMAL;
                    loop {
                        if let Err(c) = self.state.compare_exchange(
                            cur,
                            crate::client::CLOSED,
                            Ordering::SeqCst,
                            Ordering::Acquire,
                        ) {
                            if c == crate::client::NORMAL || c == crate::client::REJECT {
                                cur = c;
                                continue;
                            }
                        }
                        break;
                    }
                    (u32::MAX, crate::client::CLOSED)
                } else {
                    (self.count.load(Ordering::Acquire), crate::client::NORMAL)
                }
            }
            1 => {
                if is_closed {
                    let mut cur = crate::client::REJECT;
                    loop {
                        if let Err(c) = self.state.compare_exchange(
                            cur,
                            crate::client::CLOSED,
                            Ordering::SeqCst,
                            Ordering::Acquire,
                        ) {
                            if c == crate::client::NORMAL || c == crate::client::REJECT {
                                cur = c;
                                continue;
                            }
                        }
                        break;
                    }
                    (u32::MAX, crate::client::CLOSED)
                } else {
                    (u32::MAX, crate::client::REJECT)
                }
            }
            2 => {
                if let Err(c) = self.state.compare_exchange(
                    crate::client::CLOSED,
                    crate::client::CLEANING,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    trace_error!(state = c;"failed to compare exchange CLOSED to CLEANING");
                    (u32::MAX, c)
                } else {
                    (u32::MAX, crate::client::CLEANING)
                }
            }
            4 => (u32::MAX, crate::client::CLEANED),
            _ => (u32::MAX, crate::client::CLEANING),
        }
    }

    fn usage_add(&self) -> crate::client::StatisticsGuard {
        self.count.fetch_add(1, Ordering::AcqRel);
        crate::client::StatisticsGuard(self.count.clone())
    }
}

/// websocket
#[allow(dead_code)]
struct Ws {}

/// rpc
#[allow(dead_code)]
struct Rpc {}
