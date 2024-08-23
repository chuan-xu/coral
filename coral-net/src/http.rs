use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use axum::body::BodyDataStream;
use coral_runtime::tokio::net::ToSocketAddrs;
use coral_runtime::tokio::{self};
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use rustls::pki_types;
use rustls::ClientConfig;
use tokio_rustls::client::TlsStream;

use crate::error::CoralRes;

type HandshakeSend = hyper::client::conn::http2::SendRequest<BodyDataStream>;

type HandshakeConn = hyper::client::conn::http2::Connection<
    TokioIo<TlsStream<tokio::net::TcpStream>>,
    BodyDataStream,
    TokioExecutor,
>;
type HandshakeSocket = (HandshakeSend, HandshakeConn);

async fn http2_clien<A, D>(
    addr: A,
    tls_cfg: ClientConfig,
    // domain: pki_types::ServerName<'static>,
    domain: D,
) -> CoralRes<()>
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

// trait  {

// }

/// http2 or http3  handle of send data
pub struct HttpSendHandle<T> {
    sender: T,

    state: Arc<AtomicU8>,

    count: Arc<AtomicUsize>,
}

impl<T> HttpSendHandle<T> {
    fn new() {}
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
