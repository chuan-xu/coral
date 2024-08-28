use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use coral_runtime::tokio;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use rustls::pki_types;
use rustls::ClientConfig;
use rustls::ServerConfig;
use tokio_rustls::TlsConnector;

use crate::client::Request;
use crate::error::CoralRes;

struct TcpClient {
    remote_addr: std::net::SocketAddr,
    remote_domain: String,
    tls_cfg: ClientConfig,
}

type TlsSocket = tokio_rustls::client::TlsStream<tokio::net::TcpStream>;

async fn establish_tls_connection(
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
struct H1<B> {
    inner: hyper::client::conn::http1::SendRequest<B>,
    count: Arc<AtomicUsize>,
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
    async fn new(
        socket: TlsSocket,
        builder: hyper::client::conn::http1::Builder,
    ) -> CoralRes<Self> {
        let (sender, conn) = builder
            .handshake::<TokioIo<TlsSocket>, B>(TokioIo::new(socket))
            .await?;
        tokio::spawn(async move {
            if let Err(err) = conn.await {
                error!(e = err.to_string(); "http1 client disconnect");
            }
        });
        Ok(Self {
            inner: sender,
            count: Arc::new(AtomicUsize::default()),
        })
    }
}
impl<B> crate::client::statistics for H1<B> {
    fn usage_count(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    fn usage_add(&self) -> crate::client::StatisticsGuard {
        self.count.fetch_add(1, Ordering::AcqRel);
        crate::client::StatisticsGuard(self.count.clone())
    }
}

/// http2.0
struct H2<B> {
    inner: hyper::client::conn::http2::SendRequest<B>,
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
    async fn new(
        socket: TlsSocket,
        builder: hyper::client::conn::http2::Builder<TokioExecutor>,
    ) -> CoralRes<Self> {
        let (sender, conn) = builder
            .handshake::<TokioIo<TlsSocket>, B>(TokioIo::new(socket))
            .await?;
        tokio::spawn(async move {
            if let Err(err) = conn.await {
                error!(e = err.to_string(); "http2 client disconnect");
            }
        });
        Ok(Self { inner: sender })
    }
}

/// websocket
struct Ws {}

/// rpc
struct Rpc {}

struct TcpServer {
    listen_addr: std::net::SocketAddr,
    tls_cfg: ServerConfig,
}

/// use sample vector
struct VecClients<T, R, H> {
    inner: tokio::sync::RwLock<Vec<T>>,
    phr: PhantomData<R>,
    phh: PhantomData<H>,
}

#[async_trait::async_trait]
impl<T, R, H> crate::client::Pool for VecClients<T, R, H>
where
    T: crate::client::Request<R, H> + crate::client::statistics + Clone + Send + Sync,
    R: Send + Sync,
    H: Send + Sync,
{
    type Client = T;

    async fn load_balance(self: Arc<Self>) -> CoralRes<Option<Self::Client>> {
        let pool = self.inner.read().await;
        let mut min = usize::MAX;
        let mut instance = None;
        for item in pool.iter() {
            let count = item.usage_count();
            if count < min {
                min = count;
                instance = Some(item.clone());
            }
        }
        Ok(instance)
    }
}
