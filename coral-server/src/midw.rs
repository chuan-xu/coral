use axum::extract::Request;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::response::Response;
use log::error;
use log::warn;
use tower::Layer;
use tower::Service;

#[derive(Clone)]
pub(crate) struct EntryLayer {}

impl EntryLayer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl<S> Layer<S> for EntryLayer {
    type Service = EntryWare<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EntryWare { inner }
    }
}

#[derive(Clone)]
pub struct EntryWare<S> {
    inner: S,
}

// fn record_trace(headers: &HeaderMap<HeaderValue>) -> Option<Span> {
//     if let Some(hv) = headers.get("x-trace-id") {
//         if let Ok(trace_id) = hv.to_str() {
//             return Some(span!(Level::INFO, "trace_id", v = trace_id.to_string()));
//         } else {
//             warn!("missing trace id");
//         }
//     }
//     None
// }

impl<S> Service<Request> for EntryWare<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // let tspan = record_trace(req.headers());
        // let fut = self.inner.call(req);
        // match tspan {
        //     Some(t) => Box::pin(async move {
        //         let _guard = t.enter();
        //         fut.await
        //     }),
        //     // None => self.inner.call(req),
        //     None => Box::pin(async move { fut.await }),
        // }
        let fut = self.inner.call(req);
        Box::pin(async move { fut.await })
    }
}
