//! tower middleware
use std::future::Future;
use std::pin::Pin;

use axum::extract::Request;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use fastrace::future::FutureExt;
use fastrace::prelude::*;
use tower::Layer;
use tower::Service;

use super::HTTP_HEADER_TRACE_ID;
use crate::HTTP_HEADER_SPAN_ID;

/// midware for add trace id
#[derive(Clone)]
pub struct TraceMidware<S> {
    inner: S,
}

impl<S> Service<Request> for TraceMidware<S>
where
    S: Service<Request> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let header = req.headers_mut();
        let res = match header.get(HTTP_HEADER_TRACE_ID) {
            Some(val) => val
                .to_str()
                .map(|v| TraceId::from(CoralTraceId::from(v)))
                .map_err(|err| {
                    log::error!(e = err.to_string(); "failed to convert trace header value to str");
                }),
            None => {
                let trace_id = uuid::Uuid::new_v4().to_string();
                HeaderValue::from_str(trace_id.as_str()).map(|v| {
                    header.insert(HTTP_HEADER_TRACE_ID, v);
                    TraceId::from(CoralTraceId::from(trace_id.as_str()))
                }).map_err(|err| {
                        log::error!(e = err.to_string(); "failed to convert uuid bytes to header value");
                    })
            }
        };
        let span_id = match header.get(HTTP_HEADER_SPAN_ID) {
            Some(val) => match val.to_str() {
                Ok(v) => match v.parse::<u64>() {
                    Ok(id) => Some(SpanId(id)),
                    Err(err) => {
                        log::error!(e = err.to_string(); "failed to convert span header str to u64");
                        None
                    }
                },
                Err(err) => {
                    log::error!(e = err.to_string(); "failed to convert span header value to str");
                    None
                }
            },
            None => None,
        };
        if let Ok(trace_id) = res {
            let mut root_ctx = SpanContext::random();
            root_ctx.trace_id = trace_id;
            if let Some(id) = span_id {
                root_ctx.span_id = id;
            }
            let root_span = Span::root("trace_id", root_ctx);

            let fut = self.inner.call(req).in_span(root_span);
            Box::pin(async move { fut.await })
        } else {
            let fut = self.inner.call(req);
            Box::pin(async move { fut.await })
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

pub fn add_header_span_id(header: &mut HeaderMap) {
    if let Some(span) = SpanContext::current_local_parent() {
        header.insert(HTTP_HEADER_SPAN_ID, HeaderValue::from(span.span_id.0));
    }
}
