use coral_runtime::tokio;
use futures::{SinkExt, StreamExt};
use hyper_util::rt::TokioIo;
use log::error;
use std::sync::OnceLock;

use axum::{
    body::Body,
    http::{uri::PathAndQuery, HeaderValue},
};
use hyper::{
    body::Incoming,
    header::{CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE},
    Method, Request, Response, StatusCode, Uri,
};

use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;
use tower::Service;

use crate::{
    error::{CoralRes, Error},
    util::reset_uri_path,
    HTTP_HEADER_WEBSOCKET_CONNECTION, HTTP_HEADER_WEBSOCKET_UPGRADE,
};

pub static HTTP_RESET_URI: &'static str = "/reset";
pub static WS_RESET_URI: &'static str = "/reset_ws";

// static FRONT_ROOT: OnceLock<&str> = OnceLock::new();
static FRONT_ROOT: &'static str = "/root/web/dist/";

pub async fn front_static() -> &'static str {
    println!("debug--");
    "hello world"
}

/// Redirect h2 request
pub fn redirect_h2(
    req: hyper::Request<Incoming>,
    mut router: axum::Router,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    let headers = req.headers();
    if headers
        .get(CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase() == HTTP_HEADER_WEBSOCKET_CONNECTION)
        .unwrap_or(false)
        && headers
            .get(UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase() == HTTP_HEADER_WEBSOCKET_UPGRADE)
            .unwrap_or(false)
        && headers.get(SEC_WEBSOCKET_KEY).is_some()
        && req.method() == Method::GET
    {
        let mut reqc = Request::<Body>::default();
        *reqc.version_mut() = req.version();
        *reqc.headers_mut() = req.headers().clone();
        *(reqc.uri_mut()) = Uri::from_static(WS_RESET_URI);
        tokio::spawn(websocket_conn_hand(req));
        router.call(reqc)
    } else {
        // redirect_router(req, router, HTTP_RESET_URI)
        router.call(req)
    }
}

pub fn redirect_router(
    mut req: hyper::Request<Incoming>,
    mut router: axum::Router,
    path: &str,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    redirect_req(&mut req, path);
    router.call(req)
}

/// Redirect requests to `path`
pub fn redirect_req<T>(req: &mut hyper::Request<T>, path: &str) {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|v| v.to_owned())
        .unwrap_or(PathAndQuery::from_static("/"));
    if let Ok(uri) = reset_uri_path(req.uri(), path) {
        *req.uri_mut() = uri;
    }
    req.extensions_mut().insert(path_and_query);
}

// TODO
pub async fn websocket_conn_hand(mut req: Request<Incoming>) {
    match hyper::upgrade::on(&mut req).await {
        Ok(io) => {
            let stream = TokioIo::new(io);
            let ws_stream = WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
            let (mut outgoing, mut incoming) = ws_stream.split();
            if let Some(res) = incoming.next().await {
                match res {
                    Ok(msg) => {
                        if let Err(_e) = outgoing.send(msg).await {}
                        if let Err(_e) = outgoing.flush().await {}
                    }
                    Err(err) => error!(e = format!("{:?}", err); "failed to receive websocket mes"),
                }
            }
        }
        Err(err) => {
            error!(e = format!("{:?}", err); "fail to upgrade in websocket stream");
        }
    }
}

pub async fn websocket_upgrade_hand(req: Request<axum::body::Body>) -> CoralRes<Response<Body>> {
    let key = req
        .headers()
        .get(SEC_WEBSOCKET_KEY)
        .ok_or(Error::MissingHeader("sec-websocket-key"))?;
    let derived_key = derive_accept_key(key.as_bytes());
    let derived_hv = HeaderValue::from_str(&derived_key)?;
    let mut res = Response::new(Body::default());
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = req.version();
    res.headers_mut().append(
        CONNECTION,
        HeaderValue::from_static(HTTP_HEADER_WEBSOCKET_CONNECTION),
    );
    res.headers_mut().append(
        UPGRADE,
        HeaderValue::from_static(HTTP_HEADER_WEBSOCKET_UPGRADE),
    );
    res.headers_mut().append(SEC_WEBSOCKET_ACCEPT, derived_hv);
    Ok(res)
}

pub async fn dist(req: hyper::Request<Incoming>) {
    // req.uri().path_and_query().unwrap().path()
}

pub fn assets_router() -> axum::Router {
    let server_dir = tower_http::services::fs::ServeDir::new("/root/web/webpack/dist");
    let compress = tower_http::compression::CompressionLayer::new().no_deflate();
    axum::Router::new()
        .nest_service("/assets", server_dir)
        .layer(compress)
        .layer(tower_http::decompression::DecompressionLayer::new())
}
