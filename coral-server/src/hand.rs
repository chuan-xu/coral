use axum::extract::Request;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::routing::post;
use coral_log::log::info;
use http_body_util::BodyExt;
use hyper::StatusCode;

#[allow(unused)]
use crate::error::CoralErr;
use crate::midw::EntryLayer;

/// 健康检查
async fn heartbeat() -> hyper::Response<axum::body::Body> {
    (StatusCode::OK).into_response()
}

async fn test_hand(req: Request) -> &'static str {
    println!("headers: {:?}", req.headers());
    let (_, body) = req.into_parts();
    let c = body.collect().await.unwrap().to_bytes();
    let d = c.as_ref();
    let f = std::str::from_utf8(d).unwrap();
    println!("data {:?}", f);
    "ok!"
}

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

async fn benchmark() -> BenchmarkRes {
    info!("benchmark");
    BenchmarkRes {}
}

pub fn app() -> axum::Router {
    let entry_layer = EntryLayer::new();
    axum::Router::new()
        .route("/heartbeat", post(heartbeat))
        .route("/testhand", post(test_hand))
        .route("/benchmark", post(benchmark))
        .layer(entry_layer)
}
