//! tower middleware
use axum::extract::Request;
use axum::http::HeaderValue;
use fastrace::prelude::*;
use tower::Layer;
use tower::Service;

use super::consts::HTTP_HEADER_TRACE_ID;

/// midware for add trace id
#[derive(Clone)]
pub struct TraceMidware<S> {
    inner: S,
}

impl<S> Service<Request> for TraceMidware<S>
where
    S: Service<Request>,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        if req.uri().path() == "/heartbeat" {
            return self.inner.call(req);
        }
        let header = req.headers_mut();
        let res = match header.get(HTTP_HEADER_TRACE_ID) {
            Some(val) => val
                .to_str()
                .map(|v| TraceId::from(CoralTraceId::from(v)))
                .map_err(|err| {
                    let e_str = err.to_string();
                    log::error!(e = e_str.as_str(); "failed to convert trace header value to str");
                }),
            None => {
                let trace_id = uuid::Uuid::new_v4().to_string();
                // let trace_str = trace_id.to_string();
                HeaderValue::from_str(trace_id.as_str()).map(|v| {
                    header.insert(HTTP_HEADER_TRACE_ID, v);
                    TraceId::from(CoralTraceId::from(trace_id.as_str()))
                }).map_err(|err| {
                        let e_str = err.to_string();
                        log::error!(e = e_str.as_str(); "failed to convert uuid bytes to header value");
                    })
            }
        };
        if let Ok(trace_id) = res {
            let mut root_ctx = SpanContext::random();
            root_ctx.trace_id = trace_id;
            let root_span = Span::root("trace_id", root_ctx);
            let _guard = root_span.set_local_parent();
            self.inner.call(req)
        } else {
            self.inner.call(req)
        }
    }
}

#[derive(Clone, Default)]
pub struct TraceLayer;

impl<S> Layer<S> for TraceLayer {
    type Service = TraceMidware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service { inner }
    }
}

struct CoralTraceId(u128);

impl std::convert::From<CoralTraceId> for TraceId {
    fn from(value: CoralTraceId) -> Self {
        Self(value.0)
    }
}

impl<'a> std::convert::From<&'a str> for CoralTraceId {
    fn from(value: &'a str) -> Self {
        let mut u128byte = [0u8; 16];
        let byte = value.as_bytes();
        u128byte
            .iter_mut()
            .zip(byte.iter())
            .for_each(|(v0, v1)| *v0 = *v1);
        Self(u128::from_be_bytes(u128byte))
    }
}
