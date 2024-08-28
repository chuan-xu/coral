use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::body::BodyDataStream;
use coral_macro::trace_error;
use coral_runtime::tokio::net::ToSocketAddrs;
use coral_runtime::tokio::{self};
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use rustls::pki_types;
use rustls::ClientConfig;
use tokio_rustls::client::TlsStream;

use crate::discover::Discover;
use crate::error::CoralRes;
use crate::error::Error;

/// client is normal
static NORMAL: u8 = 0;

/// server has reject
static REJECT: u8 = 1;

/// server is closed
static CLOSED: u8 = 2;

/// handle is cleaning
static CLEANING: u8 = 3;

/// handle has cleaned
static CLEANED: u8 = 4;

type HandshakeSend = hyper::client::conn::http2::SendRequest<BodyDataStream>;
type HandshakeConn = hyper::client::conn::http2::Connection<
    TokioIo<TlsStream<tokio::net::TcpStream>>,
    BodyDataStream,
    TokioExecutor,
>;
type HandshakeSocket = (HandshakeSend, HandshakeConn);

async fn http2_clien<A, D>(addr: A, tls_cfg: ClientConfig, domain: D) -> CoralRes<HandshakeSend>
where
    A: ToSocketAddrs,
    D: TryInto<pki_types::ServerName<'static>, Error = pki_types::InvalidDnsNameError>,
{
    let tcp_stream = tokio::net::TcpStream::connect(addr).await?;
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(tls_cfg));
    let domain = domain.try_into()?;
    let tls_stream = tls_connector.connect(domain, tcp_stream).await?;
    let socket: HandshakeSocket = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(TokioIo::new(tls_stream))
        .await
        .unwrap();
    let (send, conn) = socket;
    tokio::spawn(async move {
        if let Err(err) = conn.await {
            error!(e = err.to_string(); "http2 client disconnect");
        }
    });
    Ok(send)
}

#[derive(Default)]
pub struct Http2Handle(Option<HandshakeSend>);

#[async_trait::async_trait]
impl HttpSend for Http2Handle {
    type Sender = HandshakeSend;

    async fn init(&mut self, addr: &str, tls_cfg: ClientConfig) -> CoralRes<()> {
        let (domain, _) = addr.rsplit_once(":").unwrap();
        let send = http2_clien(addr, tls_cfg, domain.to_owned()).await?;
        self.0 = Some(send);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        if let Some(this) = self.0.as_ref() {
            return this.is_closed();
        }
        true
    }

    async fn heartbeat(&self) -> CoralRes<()> {
        let body = axum::body::Body::empty().into_data_stream();
        let req = hyper::Request::builder()
            .method("POST")
            .uri("/heartbeat")
            .body(body)?;
        if let Some(sender) = self.0.as_ref() {
            let res = sender.clone().send_request(req).await?;
            if res.status() != hyper::StatusCode::OK {
                return Err(Error::HeartBeatFailed);
            }
        }

        Ok(())
    }
}

async fn http3_client<A>(
    addr: std::net::SocketAddr,
    tls_cfg: ClientConfig,
    server_name: &str,
) -> CoralRes<()> {
    let crypt = quinn::crypto::rustls::QuicClientConfig::try_from(tls_cfg)?;
    let cfg = quinn::ClientConfig::new(Arc::new(crypt));
    let endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse()?)?;
    let conn = endpoint.connect(addr, server_name)?.await?;
    let quinn_conn = h3_quinn::Connection::new(conn);
    let (driver, sender) = h3::client::new(quinn_conn).await?;
    Ok(())
}

#[async_trait::async_trait]
trait HttpSend {
    type Sender;

    async fn init(&mut self, addr: &str, tls_cfg: ClientConfig) -> CoralRes<()>;

    fn is_closed(&self) -> bool;

    async fn heartbeat(&self) -> CoralRes<()>;
}

/// http2 or http3  handle of send data
pub struct HttpSendHandle<T> {
    sender: Arc<T>,

    state: Arc<AtomicU8>,

    count: Arc<AtomicU32>,

    addr: Arc<String>,
}

impl<T> Clone for HttpSendHandle<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            state: self.state.clone(),
            count: self.count.clone(),
            addr: self.addr.clone(),
        }
    }
}

struct SenderGuard(Arc<AtomicU32>);

impl Drop for SenderGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl<T> HttpSendHandle<T>
where T: HttpSend + Default
{
    async fn new(addr: &str, tls_cfg: ClientConfig) -> CoralRes<Self> {
        let mut t = T::default();
        t.init(addr, tls_cfg).await?;
        Ok(Self {
            sender: Arc::new(t),
            state: Arc::new(AtomicU8::default()),
            count: Arc::new(AtomicU32::default()),
            addr: Arc::new(addr.to_owned()),
        })
    }

    fn get_sender(&self) -> (Arc<T>, SenderGuard) {
        self.count.fetch_add(1, Ordering::AcqRel);

        (self.sender.clone(), SenderGuard(self.count.clone()))
    }

    fn check(&self) -> (bool, u32) {
        let is_closed = self.sender.is_closed();
        let mut remove = false;
        let count = match self.state.load(Ordering::Acquire) {
            0 | 1 => {
                if is_closed {
                    let mut cur = NORMAL;
                    loop {
                        if let Err(c) = self.state.compare_exchange(
                            cur,
                            CLOSED,
                            Ordering::SeqCst,
                            Ordering::Acquire,
                        ) {
                            if c == NORMAL || c == REJECT {
                                cur = c;
                                continue;
                            }
                        }
                        break;
                    }
                    u32::MAX
                } else {
                    self.count.load(Ordering::Acquire)
                }
            }
            2 => {
                if let Err(c) = self.state.compare_exchange(
                    CLOSED,
                    CLEANING,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    trace_error!(state = c;"failed to compare exchange CLOSED to CLEANING");
                }
                u32::MAX
            }
            4 => {
                remove = true;
                u32::MAX
            }
            _ => u32::MAX,
        };
        (remove, count)
    }

    fn is_repeat(&self, addr: &str) -> bool {
        self.state.load(Ordering::Acquire) == NORMAL && *self.addr == addr
    }
}

// #[derive(Default)]
pub struct HttpSendPool<T> {
    inner: Arc<tokio::sync::RwLock<Vec<HttpSendHandle<T>>>>,
    tls_cfg: Arc<ClientConfig>,
}

impl<T> Clone for HttpSendPool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            tls_cfg: self.tls_cfg.clone(),
        }
    }
}

impl<T> HttpSendPool<T>
where T: HttpSend + Default + Send + Sync + 'static
{
    async fn remove(self) {
        let mut pool = self.inner.write().await;
        pool.retain(|item| item.state.load(Ordering::Acquire) != CLEANED);
    }

    async fn get(&self) -> Option<HttpSendHandle<T>> {
        let mut min = u32::MAX;
        let mut handle = None;
        let pool = self.inner.read().await;
        for item in pool.iter() {
            let (remove, count) = item.check();
            if remove {
                tokio::spawn(self.clone().remove());
            } else if count < min {
                min = count;
                handle = Some(item.clone());
            }
        }
        handle
    }

    async fn add(&mut self, addr: &str) {
        let cfg = (*self.tls_cfg).clone();
        match HttpSendHandle::<T>::new(addr, cfg).await {
            Ok(h) => {
                let mut pool = self.inner.write().await;
                if !pool.iter().any(|x| x.is_repeat(addr)) {
                    pool.push(h);
                }
            }
            Err(err) => {
                error!(e = err.to_string(); "failed to new http send handle");
            }
        }
    }
}

pub fn http_endpoints_discover<T>(
    addr: Vec<String>,
    mut pool: HttpSendPool<T>,
) -> std::pin::Pin<Box<impl std::future::Future<Output = ()>>>
where
    T: HttpSend + Default + Send + Sync + 'static,
{
    let fut = async move {
        for i in addr.iter() {
            pool.add(i).await;
        }
    };
    Box::pin(fut)
}
