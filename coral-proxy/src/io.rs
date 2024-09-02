use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::http::uri::PathAndQuery;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use bytes::Bytes;
use coral_runtime::tokio;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::header::CONNECTION;
use hyper::header::SEC_WEBSOCKET_KEY;
use hyper::header::UPGRADE;
use hyper::Method;
use hyper::Uri;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use log::info;
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::cli;
use crate::error::CoralRes;
use crate::http::http_reset;
use crate::http::set_discover;
use crate::http::ConnPool;
use crate::http::{self};
use crate::util;
use crate::util::reset_uri_path;
use crate::util::WS_RESET_URI;
use crate::ws;
use crate::ws::websocket_conn_hand;

fn handle_request(
    req: hyper::Request<Incoming>,
    pxy_pool: ConnPool,
    mut router: Router,
    addr: SocketAddr,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    let headers = req.headers();

    // 判断是否是websocket连接
    if headers
        .get(CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase() == "upgrade")
        .unwrap_or(false)
        && headers
            .get(UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase() == "websocket")
            .unwrap_or(false)
        && headers.get(SEC_WEBSOCKET_KEY).is_some()
        && req.method() == Method::GET
    {
        let mut reqc = Request::<Body>::default();
        *reqc.version_mut() = req.version();
        *reqc.headers_mut() = req.headers().clone();
        *(reqc.uri_mut()) = Uri::from_static(WS_RESET_URI);
        tokio::spawn(websocket_conn_hand(req, addr));
        router.call(reqc)
    } else {
        http_reset(req, pxy_pool, router)
    }
}

static RESET_URI: &'static str = "/reset";

fn map_req(mut req: hyper::Request<()>) -> hyper::Request<()> {
    let path = req
        .uri()
        .path_and_query()
        .map(|v| v.to_owned())
        .unwrap_or(PathAndQuery::from_static("/"));
    if let Ok(uri) = reset_uri_path(req.uri(), RESET_URI) {
        *req.uri_mut() = uri;
    }
    req.extensions_mut().insert(path);
    req
}

pub type T = coral_net::udp::H3;
pub type R = axum::body::Body;
pub type H = coral_net::udp::H3ClientRecv<h3_quinn::RecvStream>;

async fn server(args: &cli::Cli) -> CoralRes<()> {
    let pool = coral_net::client::VecClients::<T, R, H>::default();
    let map_req_fn = move |mut req: hyper::Request<()>| -> hyper::Request<()> {
        let pool = pool.clone();
        let path = req
            .uri()
            .path_and_query()
            .map(|v| v.to_owned())
            .unwrap_or(PathAndQuery::from_static("/"));
        if let Ok(uri) = reset_uri_path(req.uri(), RESET_URI) {
            *req.uri_mut() = uri;
        }
        req.extensions_mut().insert(pool);
        req.extensions_mut().insert(path);
        req
    };
    let addr = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        args.server_param.port,
    );
    let app = axum::Router::new().route(RESET_URI, get(|| async { "hello" }));
    coral_net::server::ServerBuiler::new(addr, coral_net::tls::server_conf(&args.tls_param)?)
        .udp_server::<_>(app, map_req_fn, None)
        .await?;
    Ok(())
}

async fn proxy(req: hyper::Request<Full<Bytes>>) -> &'static str {
    "hello from proxy"
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-proxy")?;
    if let Err(err) = rt.block_on(server(&args)) {
        error!(e = err.to_string(); "block on server {:?}", args);
    }
    Ok(())
}
