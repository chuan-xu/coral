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
use rustls::pki_types;
use rustls::ClientConfig;
use tokio_rustls::client::TlsStream;

use crate::error::CoralRes;

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

async fn http2_clien<A, D>(addr: A, tls_cfg: ClientConfig, domain: D) -> CoralRes<()>
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
    Ok(())
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

    fn get_sender(&self) -> Self::Sender;

    fn is_closed(&self) -> bool;

    async fn keep() {}

    async fn heartbeat() {}
}

/// http2 or http3  handle of send data
pub struct HttpSendHandle<T> {
    sender: Arc<T>,

    state: Arc<AtomicU8>,

    count: Arc<AtomicU32>,
}

impl<T> Clone for HttpSendHandle<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            state: self.state.clone(),
            count: self.count.clone(),
        }
    }
}

impl<T> HttpSendHandle<T>
where
    T: HttpSend,
{
    fn new() {}

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
}

#[derive(Default)]
pub struct HttpSendPool<T> {
    inner: Arc<tokio::sync::RwLock<Vec<HttpSendHandle<T>>>>,
}

impl<T> Clone for HttpSendPool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> HttpSendPool<T>
where
    T: HttpSend + Send + Sync + 'static,
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
}
