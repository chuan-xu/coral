use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::future::RouteFuture;
use bytes::Bytes;
use clap::Args;
use coral_macro::trace_error;
use coral_runtime::tokio::net::TcpStream;
use coral_runtime::tokio::{self};
use h3::quic::BidiStream;
use h3::quic::RecvStream;
use h3::server::RequestStream;
use http_body_util::BodyStream;
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

#[derive(Args, Debug)]
pub struct ServerParam {
    #[arg(long, help = "server port")]
    pub port: u16,
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
    }
}

async fn tcp_server<F>(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    peer_addr: SocketAddr,
    router: axum::Router,
    map_req: Option<F>,
) where
    F: Fn(hyper::Request<hyper::body::Incoming>, axum::Router) -> RouteFuture<Infallible> + Clone,
{
    let peer_addr = peer_addr.clone();
    match acceptor.accept(stream).await {
        Ok(stream) => {
            let service = hyper::service::service_fn(|req: hyper::Request<_>| {
                if let Some(f) = map_req.clone().take() {
                    // router.clone().call(f(req))
                    f(req, router.clone())
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

#[derive(Clone)]
pub struct H3Server<F> {
    endpoints: quinn::Endpoint,
    map_req_fn: F,
    router: axum::Router,
}

impl<F> H3Server<F>
where F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static
{
    pub async fn create_h3_client(
        self,
        peer_addr: SocketAddr,
        domain: &str,
        keep_server: bool,
    ) -> CoralRes<crate::udp::H3Sender> {
        let conn = self.endpoints.connect(peer_addr, domain)?.await?;
        if keep_server {
            let (h3_conn, sender) = h3::server::builder()
                .build_with_sender(h3_quinn::Connection::new(conn))
                .await?;
            tokio::spawn(self.quic_server(h3_conn, sender.clone()));
            Ok(sender)
        } else {
            let (mut driver, sender) = h3::client::new(h3_quinn::Connection::new(conn)).await?;
            tokio::spawn(async move {
                if let Err(err) = driver.wait_idle().await {
                    error!(e = format!("{:?}", err); "failed to run quic driver");
                }
            });
            Ok(sender)
        }
    }

    pub async fn run_server(self) -> CoralRes<()> {
        while let Some(new_conn) = self.endpoints.accept().await {
            let this = self.clone();
            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        match h3::server::builder()
                            .build_with_sender(h3_quinn::Connection::new(conn))
                            .await
                        {
                            Ok((h3_conn, sender)) => this.quic_server(h3_conn, sender).await,
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

    // async fn quic_server(self, conn: quinn::Connection) {
    async fn quic_server(
        self,
        mut h3_conn: h3::server::Connection<h3_quinn::Connection, Bytes>,
        sender: h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    ) {
        loop {
            match h3_conn.accept().await {
                Ok(Some((mut req, stream))) => {
                    req.extensions_mut().insert(sender.clone());
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
                // quic_handle_response(&mut tx, router.call(new_req).await.unwrap()).await
                quic_handle_response(&mut tx, router.call(new_req)).await
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
    // rsp: axum::response::Response,
    rsp: RouteFuture<Infallible>,
) -> CoralRes<()>
where
    T: h3::quic::SendStream<Bytes>,
{
    let rsp = rsp.await?;
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

pub struct ServerBuiler {
    addr: SocketAddr,
    server_tls: ServerConfig,
    router: Option<axum::Router>,
    client_tls: Option<ClientConfig>,
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

    pub fn h3_server<F>(
        mut self,
        mut transport_config: Option<Arc<quinn_proto::TransportConfig>>,
        map_req_fn: F,
    ) -> CoralRes<H3Server<F>>
    where
        F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static,
    {
        let mut serv_cfg = quinn::ServerConfig::with_crypto(Arc::new(
            quinn_proto::crypto::rustls::QuicServerConfig::try_from(self.server_tls.clone())?,
        ));
        if let Some(conf) = transport_config.as_ref() {
            serv_cfg.transport_config(conf.clone());
        }
        let mut endpoints = quinn::Endpoint::server(serv_cfg, self.addr)?;
        if let Some(c) = self.client_tls.take() {
            let mut client_cfg = quinn::ClientConfig::new(Arc::new(
                quinn::crypto::rustls::QuicClientConfig::try_from(c.clone())?,
            ));
            if let Some(conf) = transport_config.as_ref() {
                client_cfg.transport_config(conf.clone());
            }
            endpoints.set_default_client_config(client_cfg);
        }
        let router = self.router.take().ok_or(crate::error::Error::MissRouter)?;
        Ok(H3Server {
            endpoints,
            map_req_fn,
            router,
        })
    }

    pub async fn h2_server<F>(mut self, map_req: Option<F>) -> CoralRes<()>
    where F: Fn(hyper::Request<hyper::body::Incoming>, axum::Router) -> RouteFuture<Infallible>
            + Send
            + Sync
            + Clone
            + 'static {
        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(self.server_tls));
        let router = self.router.take().ok_or(crate::error::Error::MissRouter)?;
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let acceptor = tls_acceptor.clone();

                    let peer_addr = peer_addr.clone();
                    let map_req = map_req.clone();
                    tokio::spawn(tcp_server(
                        acceptor,
                        stream,
                        peer_addr,
                        router.clone(),
                        map_req,
                    ));
                }
                Err(err) => {
                    error!(e = format!("{:?}", err); "failed to tcp listen accept");
                }
            }
        }
        // Ok(())
    }
}
