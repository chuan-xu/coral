use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bytes::Buf;
use bytes::Bytes;
use coral_macro::trace_error;
use coral_runtime::tokio;
use h3::quic::RecvStream;
use http_body_util::BodyStream;
use hyper::Version;
use log::error;
use rustls::ClientConfig;
use tokio_stream::StreamExt;

use crate::error::CoralRes;

type H3Sender = h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>;

pub struct H3 {
    inner: h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    count: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
}

impl Clone for H3 {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            count: self.count.clone(),
            state: self.state.clone(),
        }
    }
}

impl crate::client::Statistics for H3 {
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

pub async fn create_sender_by_conf(
    addr: SocketAddr,
    domain: &str,
    tls_conf: Arc<ClientConfig>,
) -> CoralRes<H3Sender> {
    let client_conf = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_conf)?,
    ));
    let mut client_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse()?)?;
    client_endpoint.set_default_client_config(client_conf);
    let conn = client_endpoint.connect(addr, domain)?.await?;
    create_sender_by_connection(conn).await
}

pub async fn create_sender_by_endpoint(
    endpoint: quinn::Endpoint,
    addr: SocketAddr,
    domain: &str,
) -> CoralRes<H3Sender> {
    let conn = endpoint.connect(addr, domain)?.await?;
    create_sender_by_connection(conn).await
}

pub async fn create_sender_by_connection(conn: quinn::Connection) -> CoralRes<H3Sender> {
    let quinn = h3_quinn::Connection::new(conn);
    let (mut driver, sender) = h3::client::new(quinn).await?;
    tokio::spawn(async move {
        if let Err(err) = futures::future::poll_fn(|cx| driver.poll_close(cx)).await {
            error!(e = err.to_string(); "failed to run quic driver");
        }
    });
    Ok(sender)
}

impl H3 {
    pub async fn new_by_conf(
        addr: SocketAddr,
        domain: &str,
        tls_conf: Arc<ClientConfig>,
    ) -> CoralRes<Self> {
        Ok(Self::new_with_sender(
            create_sender_by_conf(addr, domain, tls_conf).await?,
        ))
    }

    pub async fn new_by_connection(conn: quinn::Connection) -> CoralRes<Self> {
        Ok(Self::new_with_sender(
            create_sender_by_connection(conn).await?,
        ))
    }

    fn new_with_sender(inner: H3Sender) -> Self {
        Self {
            inner,
            count: Arc::new(AtomicU32::default()),
            state: Arc::new(AtomicU8::default()),
        }
    }
}

async fn h3_send_body<R>(
    mut body: BodyStream<R>,
    mut tx: h3::client::RequestStream<h3_quinn::SendStream<Bytes>, Bytes>,
) where
    R: hyper::body::Body<Data = Bytes> + Send + Unpin + 'static,
    // R::Data: Send + std::fmt::Debug,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::fmt::Display,
{
    while let Some(frame) = body.next().await {
        match frame {
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
                            break;
                        }
                    }
                } else {
                    trace_error!("invalid body frame from body stream");
                    break;
                }
            }
            Err(err) => {
                trace_error!(e = err.to_string(); "failed to read frame from body stream");
                break;
            }
        }
    }
}

pin_project_lite::pin_project! {
    struct H3ServerRecv<T> {
        #[pin]
        inner: h3::server::RequestStream<T, Bytes>,
    }
}

unsafe impl<T> Send for H3ServerRecv<T> {}
unsafe impl<T> Sync for H3ServerRecv<T> {}

impl<T> hyper::body::Body for H3ServerRecv<T>
where T: h3::quic::RecvStream
{
    type Data = Bytes;

    type Error = h3::Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();
        match futures::ready!(this.inner.poll_recv_data(cx))? {
            Some(buf) => {
                // FIXME: memory usage!
                let chunk = buf.chunk();
                let frame = http_body::Frame::data(Bytes::copy_from_slice(chunk));
                std::task::Poll::Ready(Some(Ok(frame)))
            }
            None => {
                let trailers = this.inner.poll_recv_trailers()?;
                match trailers {
                    Some(t) => std::task::Poll::Ready(Some(Ok(http_body::Frame::trailers(t)))),
                    None => std::task::Poll::Ready(None),
                }
            }
        }
    }
}

pin_project_lite::pin_project! {
    pub struct H3ClientRecv<T> {
        #[pin]
        inner: h3::client::RequestStream<T, Bytes>,
    }
}

unsafe impl<T> Send for H3ClientRecv<T> {}
unsafe impl<T> Sync for H3ClientRecv<T> {}

impl hyper::body::Body for H3ClientRecv<h3_quinn::RecvStream> {
    type Data = Bytes;

    type Error = String;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();
        todo!()
    }
}

#[async_trait::async_trait]
impl<R> crate::client::Request<R, H3ClientRecv<h3_quinn::RecvStream>> for H3
where
    R: hyper::body::Body<Data = Bytes> + Send + Unpin + 'static,
    // R::Data: Send + std::fmt::Debug,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + std::fmt::Display,
{
    async fn send(
        &mut self,
        req: hyper::Request<R>,
    ) -> CoralRes<hyper::Response<H3ClientRecv<h3_quinn::RecvStream>>> {
        let mut request = hyper::http::Request::builder()
            .method(req.method())
            .uri(req.uri())
            .version(Version::HTTP_3)
            .body(())?;
        let version = req.version();
        *request.headers_mut() = req.headers().clone();
        let (tx, mut rx) = self.inner.send_request(request).await?.split();
        // tokio::spawn(h3_send_body(BodyStream::new(req.into_body()), tx));
        // let rsp = rx.recv_response().await?;
        // let response = hyper::Response::builder()
        //     .status(rsp.status())
        //     .version(version)
        //     .body(H3ClientRecv { inner: rx })?;
        // Ok(response)
        todo!()
    }
}
