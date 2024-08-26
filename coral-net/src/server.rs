use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::FromRequest;
use bytes::buf::Writer;
use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use coral_runtime::tokio::net::TcpStream;
use coral_runtime::tokio::net::ToSocketAddrs;
use coral_runtime::tokio::{self};
use h3::quic::BidiStream;
use h3::quic::RecvStream;
use h3::server::RequestStream;
use http_body_util::BodyExt;
use hyper::header::CONTENT_LENGTH;
use hyper::HeaderMap;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::error::CoralRes;
use crate::tls::server_conf;

#[async_trait::async_trait]
trait HttpServ {
    async fn run(self) -> CoralRes<()>;
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

struct H1_2<A> {
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
    async fn run(self) -> CoralRes<()> {
        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(self.tls));
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let acceptor = tls_acceptor.clone();
                    let mut inject = Inject::default();
                    inject.set_peer_addr(peer_addr);
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
    struct H3Recv<T> {
        #[pin]
        inner: RequestStream<T, Bytes>,

        length: usize,
        rsize: usize,
        trailers_tx: Option<tokio::sync::oneshot::Sender<Option<HeaderMap>>>
    }
}

unsafe impl<T> Send for H3Recv<T> {}
unsafe impl<T> Sync for H3Recv<T> {}

impl<T> hyper::body::Body for H3Recv<T>
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
                *this.rsize += chunk.len();
                let frame = http_body::Frame::data(Bytes::copy_from_slice(chunk));
                std::task::Poll::Ready(Some(Ok(frame)))
            }
            None => {
                if let Some(tx) = this.trailers_tx.take() {
                    tx.send(this.inner.poll_recv_trailers()?);
                }
                std::task::Poll::Ready(None)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        let mut hint = http_body::SizeHint::default();
        hint.set_exact((self.length - self.rsize) as u64);
        hint
    }
}

struct H3 {
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
        let (tx, rx) = stream.split();
        let (trailers_tx, trailers_rx) = tokio::sync::oneshot::channel::<Option<HeaderMap>>();
        // FIXME handle error
        let length: usize = req
            .headers()
            .get(CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let h3_recv = H3Recv {
            inner: rx,
            length,
            rsize: 0,
            trailers_tx: Some(trailers_tx),
        };
        match hyper::http::Request::builder()
            .method(req.method())
            .uri(req.uri())
            .version(req.version())
            .body(h3_recv)
        {
            Ok(mut new_req) => {
                *new_req.headers_mut() = req.headers().clone();
                // new_req = new_req.with_trailers(async move {
                //     match trailers_rx.await {
                //         Ok(rx_res) => rx_res.map(|x| Ok(x)),
                //         Err(err) => Some(Err(crate::error::Error::from(err))),
                //     }
                // });
                // let bobo = new_req.body_mut();c
                match router.call(new_req).await {
                    Ok(rsp) => {
                        // rsp.with_trailers()
                        if let Err(err) = Self::handle(tx, rsp).await {
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
        mut tx: RequestStream<T, Bytes>,
        rsp: axum::response::Response,
    ) -> CoralRes<()>
    where
        T: h3::quic::SendStream<Bytes>,
    {
        // rsp.with_trailers()
        let mut rsp_parts = hyper::http::Response::builder()
            .status(rsp.status())
            .version(hyper::Version::HTTP_3)
            .body(())?;
        *rsp_parts.headers_mut() = rsp.headers().clone();
        tx.send_response(rsp_parts).await?;
        let t = rsp.body();
        use http_body_util::BodyExt;
        // let c = t.boxed();
        // http_body
        // tx.send_data()
        // tx.send_trailers()
        Ok(())
    }
}

#[async_trait::async_trait]
impl HttpServ for H3 {
    async fn run(self) -> CoralRes<()> {
        let serv_cfg = quinn::ServerConfig::with_crypto(Arc::new(
            quinn_proto::crypto::rustls::QuicServerConfig::try_from(self.tls.clone())?,
        ));
        let endpoint = quinn::Endpoint::server(serv_cfg, self.addr)?;
        while let Some(new_conn) = endpoint.accept().await {
            tokio::spawn(async {
                let mut inject = Inject::default();
                inject.set_peer_addr(new_conn.remote_address());
                let r: axum::Router =
                    axum::Router::new().route("/", axum::routing::post(http_hand));
                inject.set_router(r);
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

pub struct Builder {}
