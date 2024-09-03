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
use rustls::ClientConfig;
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
where
    A: ToSocketAddrs + Clone,
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
                error!(e = format!("{:?}", err); "failed to accept from {:?}", addr);
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
where
    A: ToSocketAddrs + Clone + Send + Sync + 'static,
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
where
    T: RecvStream,
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
                let frame = http_body::Frame::data(buf);
                std::task::Poll::Ready(Some(Ok(frame)))
            }
            None => {
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
        // FIXME: specify size
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
    where
        T: BidiStream<Bytes> + 'static,
    {
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
                            error!(e = format!("{:?}", err);"failed to handle http3 response");
                        }
                    }
                    Err(_) => {
                        error!("failed to call http3 req in router");
                    }
                }
            }
            Err(err) => {
                error!(e = format!("{:?}", err); "failed to create http request from he receiver stream");
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
                                        error!(e = format!("{:?}", err); "failed to h3 connection accept");
                                    }
                                }
                            },
                            Err(err) => {
                                error!(e = format!("{:?}", err); "failed to establish h3 connection");
                            }
                        }
                    }
                    Err(err) => {
                        error!(e = format!("{:?}", err); "failed to finish quinn new connection in async");
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

// fn tcp_handle_request<F>(
//     mut req: hyper::Request<hyper::body::Incoming>,
//     map_req: Option<F>,
// ) -> axum::routing::future::RouteFuture<std::convert::Infallible>
// where
//     F: Fn(hyper::Request<hyper::body::Incoming>) -> hyper::Request<hyper::body::Incoming>,
// {
// let mut router = ext.router.clone();
// if let Some(ends) = ext.backends.take() {
//     req.extensions_mut().insert(ends);
// }
// if let Some(f) = map_req {
//     router.call(f(req))
// } else {
//     router.call(req)
// }

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
// }

async fn tcp_server<F>(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    peer_addr: SocketAddr,
    router: axum::Router,
    // ext: Extension<T, R, H>,
    map_req: Option<F>,
) where
    F: Fn(hyper::Request<hyper::body::Incoming>) -> hyper::Request<hyper::body::Incoming> + Clone,
{
    let peer_addr = peer_addr.clone();
    match acceptor.accept(stream).await {
        Ok(stream) => {
            let service = hyper::service::service_fn(|mut req: hyper::Request<_>| {
                // TODO by map_req
                // if let Some(ends) = ext.clone().backends.take() {
                //     req.extensions_mut().insert(ends);
                // }
                if let Some(f) = map_req.clone().take() {
                    router.clone().call(f(req))
                } else {
                    router.clone().call(req)
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
            error!(e = format!("{:?}", err); "failed to accept tls stream from {:?}", peer_addr);
        }
    }
}

pub struct ServerBuiler {
    addr: SocketAddr,
    server_tls: ServerConfig,
    router: Option<axum::Router>,
    client_tls: Option<ClientConfig>,
}

#[derive(Clone)]
pub struct H3Server<F> {
    // addr: SocketAddr,
    // server_tls: ServerConfig,
    endpoints: quinn::Endpoint,
    map_req_fn: F,
    router: axum::Router,
}

impl<F> H3Server<F>
where
    F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static,
{
    pub async fn create_h3_client(
        self,
        peer_addr: SocketAddr,
        domain: &str,
        keep_server: bool,
    ) -> CoralRes<crate::udp::H3Sender> {
        let conn = self.endpoints.connect(peer_addr, domain)?.await?;
        let (mut driver, sender) = h3::client::new(h3_quinn::Connection::new(conn.clone())).await?;
        if keep_server {
            tokio::spawn(self.quic_server(conn));
        } else {
            tokio::spawn(async move {
                if let Err(err) = futures::future::poll_fn(|cx| driver.poll_close(cx)).await {
                    error!(e = format!("{:?}", err); "failed to run quic driver");
                }
            });
        }
        Ok(sender)
    }

    pub async fn run_server(self) -> CoralRes<()> {
        while let Some(new_conn) = self.endpoints.accept().await {
            let this = self.clone();
            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => this.quic_server(conn).await,
                    Err(err) => {
                        error!(e = format!("{:?}", err); "failed to finish quinn new connection in async");
                    }
                }
            });
        }
        Ok(())
    }

    async fn quic_server(self, conn: quinn::Connection) {
        match h3::server::Connection::new(h3_quinn::Connection::new(conn.clone())).await {
            Ok(mut h3_conn) => loop {
                match h3_conn.accept().await {
                    Ok(Some((mut req, stream))) => {
                        req.extensions_mut().insert(conn.clone());
                        let map_req_fn = self.map_req_fn.clone();
                        let req = map_req_fn(req);
                        let router = self.router.clone();
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
                            error!(e = format!("{:?}", err); "failed to h3 connection accept");
                            continue;
                        }
                    },
                }
            },
            Err(err) => {
                error!(e = format!("{:?}", err); "failed to establish h3 connection");
            }
        }
    }
}

impl ServerBuiler {
    pub fn new(addr: SocketAddr, tls: ServerConfig) -> Self {
        Self {
            addr,
            server_tls: tls,
            router: None,
            client_tls: None,
        }
    }

    pub fn set_client_tls(mut self, tls: ClientConfig) -> Self {
        self.client_tls = Some(tls);
        self
    }

    pub fn set_router(mut self, router: axum::Router) -> Self {
        self.router = Some(router);
        self
    }

    pub fn h3_server<F>(mut self, map_req_fn: F) -> CoralRes<H3Server<F>>
    where
        F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static,
    {
        let serv_cfg = quinn::ServerConfig::with_crypto(Arc::new(
            quinn_proto::crypto::rustls::QuicServerConfig::try_from(self.server_tls.clone())?,
        ));
        let mut endpoints = quinn::Endpoint::server(serv_cfg, self.addr)?;
        if let Some(c) = self.client_tls.take() {
            let client_cfg = quinn::ClientConfig::new(Arc::new(
                quinn::crypto::rustls::QuicClientConfig::try_from(c.clone())?,
            ));
            endpoints.set_default_client_config(client_cfg);
        }
        let router = self.router.take().ok_or(crate::error::Error::MissRouter)?;
        Ok(H3Server {
            // addr: self.addr.clone(),
            // server_tls: self.server_tls.clone(),
            endpoints,
            map_req_fn,
            router,
        })
    }

    pub async fn tcp_server<F>(self, router: axum::Router, map_req: Option<F>) -> CoralRes<()>
    where
        F: Fn(hyper::Request<hyper::body::Incoming>) -> hyper::Request<hyper::body::Incoming>
            + Clone
            + Send
            + Sync
            + 'static,
    {
        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(self.server_tls));
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let acceptor = tls_acceptor.clone();
                    let router = router.clone();
                    let peer_addr = peer_addr.clone();
                    let map_req = map_req.clone();
                    tokio::spawn(tcp_server(acceptor, stream, peer_addr, router, map_req));
                }
                Err(err) => {
                    error!(e = format!("{:?}", err); "failed to tcp listen accept");
                }
            }
        }
        // Ok(())
    }

    pub async fn udp_server<F>(
        self,
        router: axum::Router,
        map_req_fn: F,
        report_fn: Option<Box<dyn FnOnce(quinn::Endpoint)>>,
    ) -> CoralRes<()>
    where
        F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static,
    {
        let serv_cfg = quinn::ServerConfig::with_crypto(Arc::new(
            quinn_proto::crypto::rustls::QuicServerConfig::try_from(self.server_tls.clone())?,
        ));
        let endpoint = quinn::Endpoint::server(serv_cfg, self.addr)?;
        if let Some(f) = report_fn {
            let mut client_endp = endpoint.clone();
            // if let Some(c) = self.client_tls.as_ref() {
            //     let client_cfg = quinn::ClientConfig::new(Arc::new(
            //         quinn::crypto::rustls::QuicClientConfig::try_from(c.clone())?,
            //     ));
            //     client_endp.set_default_client_config(client_cfg);
            // }
            f(client_endp);
        }
        while let Some(new_conn) = endpoint.accept().await {
            let router = router.clone();
            // tokio::spawn(quic_server(new_conn, router, map_req_fn.clone()));
        }
        Ok(())
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
                trace_error!(e = format!("{:?}", err);"faild to handle response in quic");
            }
        }
        Err(err) => {
            error!(e = format!("{:?}", err); "failed to create http request from he receiver stream");
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
                                trace_error!(e = format!("{:?}", err); "failed to send data frame in quic");
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
                                trace_error!(e = format!("{:?}", err); "failed to send trailers frame in quic");
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
                trace_error!(e = format!("{:?}", err); "failed to read frame from body stream");
                tx.stop_stream(h3::error::Code::H3_INTERNAL_ERROR);
                break;
            }
        }
    }
    tx.finish().await?;
    Ok(())
}
