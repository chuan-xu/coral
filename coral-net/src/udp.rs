use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bytes::Bytes;
use coral_macro::trace_error;
use coral_runtime::spawn;
use http_body_util::BodyStream;
use hyper::Version;
use tokio_stream::StreamExt;

use crate::error::CoralRes;

pub type H3Sender = h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>;

pub struct H3 {
    inner: h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    authority: Arc<String>,
    count: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
}

impl Clone for H3 {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            authority: self.authority.clone(),
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

impl H3 {
    pub fn new_with_sender(inner: H3Sender, authority: String) -> Self {
        Self {
            inner,
            authority: Arc::new(authority),
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
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::fmt::Display,
{
    while let Some(frame) = body.next().await {
        match frame {
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
                            break;
                        }
                    }
                } else {
                    trace_error!("invalid body frame from body stream");
                    break;
                }
            }
            Err(err) => {
                trace_error!(e = format!("{}", err); "failed to read frame from body stream");
                break;
            }
        }
    }
    if let Err(err) = tx.finish().await {
        trace_error!(e = format!("{}", err); "failed to finish stream send");
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

    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();
        match futures::ready!(this.inner.poll_recv_data(cx))? {
            Some(frame) => std::task::Poll::Ready(Some(Ok(http_body::Frame::data(frame)))),
            None => {
                if let Some(trailers) = this.inner.poll_recv_trailers()? {
                    std::task::Poll::Ready(Some(Ok(http_body::Frame::trailers(trailers))))
                } else {
                    std::task::Poll::Ready(None)
                }
            }
        }
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
        let uri_bd = hyper::Uri::builder()
            .scheme("https")
            .authority(self.authority.as_str());
        let uri = match req.uri().path_and_query() {
            Some(path_query) => uri_bd.path_and_query(path_query.to_owned()).build(),
            None => uri_bd.build(),
        }?;
        let mut request = hyper::http::Request::builder()
            .method(req.method())
            .uri(uri)
            .version(Version::HTTP_3)
            .body(())?;
        let version = req.version();
        *request.headers_mut() = req.headers().clone();
        let (tx, mut rx) = self.inner.send_request(request).await?.split();
        spawn(h3_send_body(BodyStream::new(req.into_body()), tx));
        let rsp = rx.recv_response().await?;
        let mut response = hyper::Response::builder()
            .status(rsp.status())
            .version(version)
            .body(H3ClientRecv { inner: rx })?;
        *response.headers_mut() = rsp.headers().clone();
        Ok(response)
    }
}
