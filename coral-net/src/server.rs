use std::io::Write;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::FromRequest;
use bytes::buf::Writer;
use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use clap::Args;
use coral_macro::trace_error;
use coral_runtime::tokio::net::TcpStream;
use coral_runtime::tokio::net::ToSocketAddrs;
use coral_runtime::tokio::{self};
use h3::quic::BidiStream;
use h3::quic::RecvStream;
use h3::server::RequestStream;
use http_body_util::BodyExt;
use http_body_util::BodyStream;
use hyper::header::CONTENT_LENGTH;
use hyper::HeaderMap;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use log::info;
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tokio_stream::StreamExt;
use tower::Service;

use crate::error::CoralRes;
use crate::tls::server_conf;

#[derive(Args, Debug)]
pub struct ServerParam {
    #[arg(long, help = "server port")]
    pub port: u16,
}

#[async_trait::async_trait]
pub trait HttpServ {
    async fn run(self, router: axum::Router) -> CoralRes<()>;
}

#[derive(Default)]
struct Inject {
    router: Option<axum::Router>,
    peer_addr: Option<SocketAddr>,
}

impl Clone for Inject {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            peer_addr: self.peer_addr.clone(),
        }
    }
}

impl Inject {
    fn set_router(&mut self, router: axum::Router) {
        self.router = Some(router);
    }

    fn set_peer_addr(&mut self, peer_addr: SocketAddr) {
        self.peer_addr = Some(peer_addr)
    }
}

pub struct H1_2<A> {
    tls: ServerConfig,
    addr: A,
}

impl<A> H1_2<A>
where A: ToSocketAddrs + Clone
{
    async fn bind(acceptor: TlsAcceptor, stream: TcpStream, inject: Inject) {
        let addr = inject.peer_addr.clone();
        match acceptor.accept(stream).await {
            Ok(tls_stream) => {
                let service = hyper::service::service_fn(|req: hyper::Request<_>| {
                    Self::handle_req(req, inject.clone())
                });
                if let Err(err) = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(TokioIo::new(tls_stream), service)
                    .await
                {
                    error!("failed to serving connection from {:?}: {}", addr, err);
                }
            }
            Err(err) => {
                error!(e = err.to_string(); "failed to accept from {:?}", addr);
            }
        }
    }

    fn handle_req(
        req: hyper::Request<hyper::body::Incoming>,
        mut inject: Inject,
    ) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
        let router = inject.router.as_mut().unwrap();
        router.call(req)
    }
}

#[async_trait::async_trait]
impl<A> HttpServ for H1_2<A>
where A: ToSocketAddrs + Clone + Send + Sync + 'static
{
    async fn run(self, router: axum::Router) -> CoralRes<()> {
        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(self.tls));
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let acceptor = tls_acceptor.clone();
                    let mut inject = Inject::default();
                    inject.set_peer_addr(peer_addr);
                    inject.set_router(router.clone());
                    let fut = Self::bind(acceptor, stream, inject);
                    tokio::spawn(fut);
                }
                Err(_) => todo!(),
            }
        }
        Ok(())
    }
}

pin_project_lite::pin_project! {
    struct H3RecvStream<T> {
        #[pin]
        inner: RequestStream<T, Bytes>,
    }
}

unsafe impl<T> Send for H3RecvStream<T> {}
unsafe impl<T> Sync for H3RecvStream<T> {}

impl<T> hyper::body::Body for H3RecvStream<T>
where T: RecvStream
{
    type Data = Bytes;

    type Error = crate::error::Error;

    // FIXME: handle http3 error code
    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();
        // this.inner.recv_trailers()
        match futures::ready!(this.inner.poll_recv_data(cx))? {
            Some(buf) => {
                // FIXME: memory usage!
                let chunk = buf.chunk();
                // *this.rsize += chunk.len();
                let frame = http_body::Frame::data(Bytes::copy_from_slice(chunk));
                std::task::Poll::Ready(Some(Ok(frame)))
            }
            None => {
                // if let Some(tx) = this.trailers_tx.take() {
                //     if let Err(e) = tx.send(this.inner.poll_recv_trailers()?) {
                //         error!("failed to transfer trailers: {:?}", e);
                //     }
                // }
                let trailers = this.inner.poll_recv_trailers()?;
                match trailers {
                    Some(t) => std::task::Poll::Ready(Some(Ok(http_body::Frame::trailers(t)))),
                    None => std::task::Poll::Ready(None),
                }
                // std::task::Poll::Ready(None)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::default()
        // let mut hint = http_body::SizeHint::default();
        // hint.set_exact((self.length - self.rsize) as u64);
        // hint
    }
}

pub struct H3 {
    tls: ServerConfig,
    addr: SocketAddr,
}

// async fn http_hand(req: hyper::http::Request<H3Recv<_>>) {}
async fn http_hand(mut req: axum::extract::Request) {
    let body = req.body_mut();
    // body.with_trailers()
}

impl H3 {
    async fn bind<T>(req: hyper::Request<()>, stream: RequestStream<T, Bytes>, mut inject: Inject)
    where T: BidiStream<Bytes> + 'static {
        let router = inject.router.as_mut().unwrap();
        let (mut tx, rx) = stream.split();
        // FIXME: handle error
        let h3_recv = H3RecvStream { inner: rx };
        match hyper::http::Request::builder()
            .method(req.method())
            .uri(req.uri())
            .version(req.version())
            .body(h3_recv)
        {
            Ok(mut new_req) => {
                *new_req.headers_mut() = req.headers().clone();
                match router.call(new_req).await {
                    Ok(rsp) => {
                        if let Err(err) = Self::handle(&mut tx, rsp).await {
                            error!(e = err.to_string();"failed to handle http3 response");
                        }
                    }
                    Err(_) => {
                        error!("failed to call http3 req in router");
                    }
                }
            }
            Err(err) => {
                error!(e = err.to_string(); "failed to create http request from he receiver stream");
            }
        }
    }

    // <T as BidiStream<Bytes>>::SendStream
    async fn handle<T>(
        tx: &mut RequestStream<T, Bytes>,
        rsp: axum::response::Response,
    ) -> CoralRes<()>
    where
        T: h3::quic::SendStream<Bytes>,
    {
        let (parts, rsp_body) = rsp.into_parts();
        let mut rsp_parts = hyper::http::Response::builder()
            .status(parts.status)
            .version(hyper::Version::HTTP_3)
            .body(())?;
        *rsp_parts.headers_mut() = parts.headers.clone();
        tx.send_response(rsp_parts).await?;
        let mut body_stream = BodyStream::new(rsp_body);
        // INFO: use tokio StreamExt
        while let Some(body_frame) = body_stream.next().await {
            match body_frame {
                Ok(frame) => {
                    if frame.is_data() {
                        match frame.into_data() {
                            Ok(f_d) => {
                                // TODO: handle error
                                tx.send_data(f_d).await.unwrap();
                            }
                            Err(_) => todo!(),
                        }
                    } else if frame.is_trailers() {
                        match frame.into_trailers() {
                            Ok(f_t) => {
                                // TODO: handle error
                                tx.send_trailers(f_t).await.unwrap();
                            }
                            Err(_) => todo!(),
                        }
                    }
                }
                Err(err) => todo!(),
            }
        }
        // TODO: handle error
        tx.finish().await.unwrap();
        Ok(())
    }
}

#[async_trait::async_trait]
impl HttpServ for H3 {
    async fn run(self, router: axum::Router) -> CoralRes<()> {
        let serv_cfg = quinn::ServerConfig::with_crypto(Arc::new(
            quinn_proto::crypto::rustls::QuicServerConfig::try_from(self.tls.clone())?,
        ));
        let endpoint = quinn::Endpoint::server(serv_cfg, self.addr)?;
        while let Some(new_conn) = endpoint.accept().await {
            let router = router.clone();
            tokio::spawn(async move {
                let mut inject = Inject::default();
                inject.set_router(router);
                inject.set_peer_addr(new_conn.remote_address());
                match new_conn.await {
                    Ok(conn) => {
                        match h3::server::Connection::new(h3_quinn::Connection::new(conn)).await {
                            Ok(mut h3_conn) => loop {
                                match h3_conn.accept().await {
                                    Ok(Some((req, stream))) => {
                                        tokio::spawn(Self::bind(req, stream, inject.clone()));
                                    }
                                    Ok(None) => {}
                                    Err(err) => {
                                        error!(e = err.to_string(); "failed to h3 connection accept");
                                    }
                                }
                            },
                            Err(err) => {
                                error!(e = err.to_string(); "failed to establish h3 connection");
                            }
                        }
                    }
                    Err(err) => {
                        error!(e = err.to_string(); "failed to finish quinn new connection in async");
                    }
                }
            });
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Builder {
    tls_config: Option<ServerConfig>,
    http1_only: bool,
    http2_only: bool,
    http1_or_2: bool,
    address: String,
}

impl Builder {
    pub fn tls_config(mut self, config: ServerConfig) -> Self {
        self.tls_config = Some(config);
        self
    }

    pub fn http1_only(mut self, set: bool) -> Self {
        self.http1_only = set;
        self
    }

    pub fn http2_only(mut self, set: bool) -> Self {
        self.http2_only = set;
        self
    }

    pub fn http1_or_2(mut self, set: bool) -> Self {
        self.http1_or_2 = set;
        self
    }

    pub fn address(mut self, address: String) -> Self {
        self.address = address;
        self
    }

    pub fn http1_2(mut self) -> H1_2<String> {
        H1_2 {
            tls: self.tls_config.take().unwrap(),
            addr: self.address,
        }
    }

    pub fn http3(mut self) -> H3 {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9001);
        H3 {
            tls: self.tls_config.take().unwrap(),
            addr,
        }
    }
}

struct Extension<T, R, H> {
    router: axum::Router,
    peer_addr: Option<SocketAddr>,
    backends: Option<std::collections::HashMap<String, crate::client::VecClients<T, R, H>>>,
}

impl<T, R, H> Clone for Extension<T, R, H>
where T: Clone + crate::client::Statistics
{
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            peer_addr: self.peer_addr.clone(),
            backends: self.backends.clone(),
        }
    }
}

fn tcp_handle_request<T, R, H, F>(
    mut req: hyper::Request<hyper::body::Incoming>,
    mut ext: Extension<T, R, H>,
    map_req: Option<F>,
) -> axum::routing::future::RouteFuture<std::convert::Infallible>
where
    T: crate::client::Statistics + Clone + Send + Sync + 'static,
    R: Send + Sync + 'static,
    H: Send + Sync + 'static,
    F: Fn(hyper::Request<hyper::body::Incoming>) -> hyper::Request<hyper::body::Incoming>,
{
    let mut router = ext.router.clone();
    if let Some(ends) = ext.backends.take() {
        req.extensions_mut().insert(ends);
    }
    if let Some(f) = map_req {
        router.call(f(req))
    } else {
        router.call(req)
    }

    // let headers = req.headers();

    // 判断是否是websocket连接
    // if headers
    //     .get(hyper::header::CONNECTION)
    //     .and_then(|v| v.to_str().ok())
    //     .map(|v| v.to_lowercase() == "upgrade")
    //     .unwrap_or(false)
    //     && headers
    //         .get(hyper::header::UPGRADE)
    //         .and_then(|v| v.to_str().ok())
    //         .map(|v| v.to_lowercase() == "websocket")
    //         .unwrap_or(false)
    //     && headers.get(hyper::header::SEC_WEBSOCKET_KEY).is_some()
    //     && req.method() == hyper::Method::GET
    // {
    //     let mut reqc = hyper::Request::<axum::body::Body>::default();
    //     *reqc.version_mut() = req.version();
    //     *reqc.headers_mut() = req.headers().clone();
    //     *(reqc.uri_mut()) = hyper::Uri::from_static("/reset_ws");
    //     // TODO
    //     // tokio::spawn(websocket_conn_hand(req, addr));
    //     router.call(reqc)
    // } else {
    //     if let Some(f) = map_req {
    //         router.call(f(req))
    //     } else {
    //         router.call(req)
    //     }
    // }
}

async fn tcp_server<T, R, H, F>(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    ext: Extension<T, R, H>,
    map_req: Option<F>,
) where
    T: crate::client::Statistics + Clone + Send + Sync + 'static,
    R: Send + 'static,
    H: Send + 'static,
    F: Fn(hyper::Request<hyper::body::Incoming>) -> hyper::Request<hyper::body::Incoming> + Clone,
{
    let peer_addr = ext.peer_addr.clone();
    match acceptor.accept(stream).await {
        Ok(stream) => {
            let service = hyper::service::service_fn(|mut req: hyper::Request<_>| {
                let mut router = ext.router.clone();
                if let Some(ends) = ext.clone().backends.take() {
                    req.extensions_mut().insert(ends);
                }
                if let Some(f) = map_req.clone().take() {
                    router.call(f(req))
                } else {
                    router.call(req)
                }
            });
            if let Err(err) = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(stream), service)
                .await
            {
                error!("failed to serving connection from {:?}: {}", peer_addr, err);
            }
        }
        Err(err) => {
            error!(e = err.to_string(); "failed to accept tls stream from {:?}", peer_addr);
        }
    }
}

pub struct ServerBuiler<T, R, H> {
    addr: SocketAddr,
    tls: ServerConfig,
    backends: Option<std::collections::HashMap<String, crate::client::VecClients<T, R, H>>>,
}

impl<T, R, H> ServerBuiler<T, R, H>
where
    T: crate::client::Statistics + Clone + Send + Sync + 'static,
    R: Send + 'static,
    H: Send + 'static,
{
    pub fn new(addr: SocketAddr, tls: ServerConfig) -> Self {
        Self {
            addr,
            tls,
            backends: Some(std::collections::HashMap::default()),
        }
    }

    pub fn add_backend(
        mut self,
        backend_name: String,
        backend: crate::client::VecClients<T, R, H>,
    ) -> Self {
        if let Some(m) = self.backends.as_mut() {
            m.insert(backend_name, backend);
        }
        self
    }

    pub async fn tcp_server<F>(self, router: axum::Router, map_req: Option<F>) -> CoralRes<()>
    where F: Fn(hyper::Request<hyper::body::Incoming>) -> hyper::Request<hyper::body::Incoming>
            + Clone
            + Send
            + Sync
            + 'static {
        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(self.tls));
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let acceptor = tls_acceptor.clone();
                    let ext = Extension {
                        router: router.clone(),
                        peer_addr: Some(peer_addr.clone()),
                        backends: self.backends.clone(),
                    };
                    let map_req = map_req.clone();
                    tokio::spawn(tcp_server(acceptor, stream, ext, map_req));
                }
                Err(err) => {
                    error!(e = err.to_string(); "failed to tcp listen accept");
                }
            }
        }
        // Ok(())
    }

    pub async fn udp_server<F>(self, router: axum::Router, map_req: Option<F>) -> CoralRes<()>
    where F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static {
        let serv_cfg = quinn::ServerConfig::with_crypto(Arc::new(
            quinn_proto::crypto::rustls::QuicServerConfig::try_from(self.tls.clone())?,
        ));
        let endpoint = quinn::Endpoint::server(serv_cfg, self.addr)?;
        while let Some(new_conn) = endpoint.accept().await {
            let router = router.clone();
            let ext = Extension {
                router,
                peer_addr: None,
                backends: self.backends.clone(),
            };
            tokio::spawn(quic_server(new_conn, ext, map_req.clone()));
        }
        Ok(())
    }
}

async fn quic_server<F, T, R, H>(conn: quinn::Incoming, ext: Extension<T, R, H>, map_req: Option<F>)
where
    T: crate::client::Statistics + Clone + Send + Sync + 'static,
    R: Send + 'static,
    H: Send + 'static,
    // U: BidiStream<Bytes> + 'static,
    F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone,
{
    match conn.await {
        Ok(conn) => match h3::server::Connection::new(h3_quinn::Connection::new(conn)).await {
            Ok(mut h3_conn) => loop {
                match h3_conn.accept().await {
                    Ok(Some((req, stream))) => {
                        let mut req = match map_req.clone() {
                            Some(f) => f(req),
                            None => req,
                        };
                        if let Some(backends) = ext.backends.as_ref() {
                            req.extensions_mut().insert(backends.clone());
                        }
                        let router = ext.router.clone();
                        tokio::spawn(quic_handle_request(req, stream, router));
                    }
                    Ok(None) => {
                        info!("disconnect");
                        break;
                    }
                    Err(err) => match err.get_error_level() {
                        h3::error::ErrorLevel::ConnectionError => {
                            info!("disconnect");
                            break;
                        }
                        h3::error::ErrorLevel::StreamError => {
                            error!(e = err.to_string(); "failed to h3 connection accept");
                            continue;
                        }
                    },
                }
            },
            Err(err) => {
                error!(e = err.to_string(); "failed to establish h3 connection");
            }
        },
        Err(err) => {
            error!(e = err.to_string(); "failed to finish quinn new connection in async");
        }
    }
}

async fn quic_handle_request<U>(
    req: hyper::Request<()>,
    stream: RequestStream<U, Bytes>,
    mut router: axum::Router,
) where
    U: BidiStream<Bytes> + 'static,
{
    let (mut tx, rx) = stream.split();
    let h3_recv = H3RecvStream { inner: rx };
    match hyper::http::Request::builder()
        .method(req.method())
        .uri(req.uri())
        .version(req.version())
        .body(h3_recv)
    {
        Ok(mut new_req) => {
            *new_req.headers_mut() = req.headers().clone();
            *new_req.extensions_mut() = req.extensions().clone();
            if let Err(err) =
                quic_handle_response(&mut tx, router.call(new_req).await.unwrap()).await
            {
                trace_error!(e = err.to_string();"faild to handle response in quic");
            }
        }
        Err(err) => {
            error!(e = err.to_string(); "failed to create http request from he receiver stream");
        }
    }
}

async fn quic_handle_response<T>(
    tx: &mut RequestStream<T, Bytes>,
    rsp: axum::response::Response,
) -> CoralRes<()>
where
    T: h3::quic::SendStream<Bytes>,
{
    let mut parts = hyper::http::Response::builder()
        .status(rsp.status())
        .version(hyper::Version::HTTP_3)
        .body(())?;
    *parts.headers_mut() = rsp.headers().clone();
    tx.send_response(parts).await?;
    let mut body_stream = BodyStream::new(rsp.into_body());
    // INFO: use tokio StreamExt
    while let Some(body_frame) = body_stream.next().await {
        match body_frame {
            Ok(body_frame) => {
                if body_frame.is_data() {
                    match body_frame.into_data() {
                        Ok(frame) => {
                            if let Err(err) = tx.send_data(frame).await {
                                trace_error!(e = err.to_string(); "failed to send data frame in quic");
                            }
                        }
                        Err(err) => {
                            trace_error!("failed to parse data frame: {:?}", err);
                            tx.stop_stream(h3::error::Code::H3_INTERNAL_ERROR);
                            break;
                        }
                    }
                } else if body_frame.is_trailers() {
                    match body_frame.into_trailers() {
                        Ok(frame) => {
                            if let Err(err) = tx.send_trailers(frame).await {
                                trace_error!(e = err.to_string(); "failed to send trailers frame in quic");
                            }
                        }
                        Err(err) => {
                            trace_error!("failed to parse trailers frame: {:?}", err);
                            tx.stop_stream(h3::error::Code::H3_INTERNAL_ERROR);
                            break;
                        }
                    }
                } else {
                    trace_error!("invalid body frame from body stream");
                    tx.stop_stream(h3::error::Code::H3_INTERNAL_ERROR);
                    break;
                }
            }
            Err(err) => {
                trace_error!(e = err.to_string(); "failed to read frame from body stream");
                tx.stop_stream(h3::error::Code::H3_INTERNAL_ERROR);
                break;
            }
        }
    }
    tx.finish().await?;
    Ok(())
}
