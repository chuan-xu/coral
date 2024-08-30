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
use coral_runtime::tokio;
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

async fn server(args: &cli::Cli) -> CoralRes<()> {
    let pool = coral_net::client::VecClients::<_, axum::body::Body, _>::default();
    let confs = args.get_conn()?;
    for conf in confs.iter() {
        let tls_conf = coral_net::tls::client_conf(&coral_net::tls::TlsParam::new(
            conf.ca.clone(),
            conf.cert.clone(),
            conf.key.clone(),
        ))?;
        let addr = SocketAddr::new(std::net::IpAddr::from_str(&conf.ip).unwrap(), conf.port);
        let conn = coral_net::udp::H3::new(addr, &conf.domain, Arc::new(tls_conf)).await?;
        pool.clone().add(conn).await;
    }
    let addr = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        args.server_param.port,
    );
    let app = axum::Router::new().route(RESET_URI, get(|| async { "hello" }));
    coral_net::server::ServerBuiler::new(addr, coral_net::tls::server_conf(&args.tls_param)?)
        .add_backend(String::from("h3"), pool)
        .udp_server::<_>(app, Some(map_req))
        .await?;
    Ok(())
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-proxy")?;
    if let Err(err) = rt.block_on(server(&args)) {
        error!(e = err.to_string(); "block on server {:?}", args);
    }
    Ok(())
}
