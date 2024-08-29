use std::net::SocketAddr;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bytes::Buf;
use bytes::Bytes;
use coral_runtime::tokio;
use http_body_util::BodyStream;
use hyper::Version;
use log::error;
use rustls::ClientConfig;
use rustls::ServerConfig;

use crate::error::CoralRes;

struct UdpServer {
    listen_addr: std::net::SocketAddr,
    tls_cfg: ServerConfig,
}

struct H3 {
    inner: h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    count: Arc<AtomicUsize>,
    state: Arc<AtomicU8>,
}

impl H3 {
    pub async fn new(
        addr: SocketAddr,
        domain: &str,
        tls_conf: Arc<ClientConfig>,
    ) -> CoralRes<Self> {
        let client_conf = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_conf)?,
        ));
        let mut client_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse()?)?;
        client_endpoint.set_default_client_config(client_conf);
        let conn = client_endpoint.connect(addr, domain)?.await?;
        let quinn = h3_quinn::Connection::new(conn);
        let (mut driver, sender) = h3::client::new(quinn).await?;
        tokio::spawn(async move {
            if let Err(err) = futures::future::poll_fn(|cx| driver.poll_close(cx)).await {
                error!(e = err.to_string(); "failed to run quic driver");
            }
        });
        Ok(Self {
            inner: sender,
            count: Arc::new(AtomicUsize::default()),
            state: Arc::new(AtomicU8::default()),
        })
    }
}

async fn h3_send_body<R>(
    body: BodyStream<R>,
    tx: h3::client::RequestStream<h3_quinn::SendStream<Bytes>, Bytes>,
) where
    R: hyper::body::Body + Send + Unpin + 'static,
    R::Data: Send,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
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
                // std::task::Poll::Ready(None)
            }
        }
    }
}

pin_project_lite::pin_project! {
    struct H3ClientRecv<T> {
        #[pin]
        inner: h3::client::RequestStream<T, Bytes>,
    }
}

unsafe impl<T> Send for H3ClientRecv<T> {}
unsafe impl<T> Sync for H3ClientRecv<T> {}

#[async_trait::async_trait]
impl<R> crate::client::Request<R, H3ClientRecv<h3_quinn::RecvStream>> for H3
where
    R: hyper::body::Body + Send + Unpin + 'static,
    R::Data: Send,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
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
        tokio::spawn(h3_send_body(BodyStream::new(req.into_body()), tx));
        let rsp = rx.recv_response().await?;
        let response = hyper::Response::builder()
            .status(rsp.status())
            .version(version)
            .body(H3ClientRecv { inner: rx })?;
        Ok(response)
    }
}
