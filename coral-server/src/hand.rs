use axum::extract::Request;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::middleware::map_response;
use axum::response::IntoResponse;
use axum::routing::post;
use bytes::Bytes;
use coral_macro::trace_info;
use coral_runtime::spawn;
use fastrace::future::FutureExt;
use fastrace::local::LocalSpan;
use fastrace::Span;
use http_body_util::BodyExt;
use hyper::header::ALT_SVC;
use hyper::StatusCode;
use log::info;
use std::sync::OnceLock;

pub static H3_PORT: OnceLock<u16> = OnceLock::new();

/// 健康检查
async fn heartbeat() -> hyper::Response<axum::body::Body> {
    (StatusCode::OK).into_response()
}

async fn test_hand(req: Request) -> &'static str {
    let (_, body) = req.into_parts();
    let c = body.collect().await.unwrap().to_bytes();
    let d = c.as_ref();
    let _f = std::str::from_utf8(d).unwrap();
    "ok!"
}

#[allow(dead_code)]
struct BenchmarkRes {}

impl IntoResponse for BenchmarkRes {
    fn into_response(self) -> axum::response::Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("benchmark"),
            HeaderValue::from_static("benchmark"),
        );
        (headers, "benchmark").into_response()
    }
}

// #[axum::debug_handler]
async fn benchmark(header: hyper::HeaderMap, body: Bytes) -> (hyper::StatusCode, Bytes) {
    info!("benchmark");
    let size = header
        .get(hyper::header::CONTENT_LENGTH)
        .map(|v| {
            v.to_str()
                .map(|x| x.parse::<usize>().unwrap_or_default())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let code = match size == body.len() {
        true => hyper::StatusCode::OK,
        false => hyper::StatusCode::INTERNAL_SERVER_ERROR,
    };

    (code, body)
}

#[axum::debug_handler]
async fn test_trace() -> &'static str {
    trace_info!("enter test span handle");
    {
        let _span =
            LocalSpan::enter_with_local_parent("test_span").with_property(|| ("lululu", "tody"));
        parallel_job();
    }
    other_job().await;
    "record trace"
}

fn parallel_job() -> Vec<coral_runtime::tokio::task::JoinHandle<()>> {
    let mut v = Vec::with_capacity(4);
    for i in 0..4 {
        v.push(spawn(
            iter_job(i).in_span(Span::enter_with_local_parent("iter job")),
        ));
    }
    v
}

async fn iter_job(iter: u64) {
    std::thread::sleep(std::time::Duration::from_millis(iter * 10));
    coral_runtime::tokio::task::yield_now().await;
    other_job().await;
}

#[fastrace::trace(enter_on_poll = true)]
async fn other_job() {
    for i in 0..20 {
        if i == 10 {
            coral_runtime::tokio::task::yield_now().await;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

async fn alt_svc_header<B>(mut rsp: hyper::Response<B>) -> hyper::Response<B> {
    // tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    rsp.headers_mut()
        // .insert(ALT_SVC, HeaderValue::from_str(&alt_svc).unwrap());
        .insert(
            ALT_SVC,
            HeaderValue::from_static(
                "h3-27=\":443\"; ma=86400, h3-28=\":443\"; ma=86400, h3-29=\":443\"; ma=86400, h3=\":443\"; ma=86400",
            ),
        );
    rsp
}

// TODO: will test wasm protobuf in frontend
async fn test_payload(req: Request) {}

pub fn router(conf: &crate::cli::Conf) -> axum::Router {
    let router = match conf.assets.as_ref() {
        Some(f) => f.service(),
        None => axum::Router::new(),
    };
    router
        .layer(map_response(alt_svc_header))
        .route("/heartbeat", post(heartbeat))
        .route("/testhand", post(test_hand))
        .route("/benchmark", post(benchmark))
        .route("/trace", post(test_trace))
        .layer(coral_net::midware::TraceLayer::default())
}
