use std::net::Ipv4Addr;
use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::Request;
use axum::http::uri::PathAndQuery;
use axum::Router;
use coral_runtime::tokio;
use hyper::body::Incoming;
use hyper::header::CONNECTION;
use hyper::header::SEC_WEBSOCKET_KEY;
use hyper::header::UPGRADE;
use hyper::Method;
use hyper::Uri;
use log::error;
use tower::Service;

use crate::cli;
use crate::error::CoralRes;
use crate::http::RECV_ENDPOINTS;
use crate::http::RESET_URI;
use crate::util::reset_uri_path;
use crate::util::WS_RESET_URI;
use crate::ws;
use crate::ws::websocket_conn_hand;

// fn handle_request(
//     req: hyper::Request<Incoming>,
//     pxy_pool: ConnPool,
//     mut router: Router,
//     addr: SocketAddr,
// ) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
//     let headers = req.headers();

//     // 判断是否是websocket连接
//     if headers
//         .get(CONNECTION)
//         .and_then(|v| v.to_str().ok())
//         .map(|v| v.to_lowercase() == "upgrade")
//         .unwrap_or(false)
//         && headers
//             .get(UPGRADE)
//             .and_then(|v| v.to_str().ok())
//             .map(|v| v.to_lowercase() == "websocket")
//             .unwrap_or(false)
//         && headers.get(SEC_WEBSOCKET_KEY).is_some()
//         && req.method() == Method::GET
//     {
//         let mut reqc = Request::<Body>::default();
//         *reqc.version_mut() = req.version();
//         *reqc.headers_mut() = req.headers().clone();
//         *(reqc.uri_mut()) = Uri::from_static(WS_RESET_URI);
//         tokio::spawn(websocket_conn_hand(req, addr));
//         router.call(reqc)
//     } else {
//         // http_reset(req, pxy_pool, router)
//         router.call(req)
//     }
// }

// fn map_req(mut req: hyper::Request<()>) -> hyper::Request<()> {
//     let path = req
//         .uri()
//         .path_and_query()
//         .map(|v| v.to_owned())
//         .unwrap_or(PathAndQuery::from_static("/"));
//     if let Ok(uri) = reset_uri_path(req.uri(), RESET_URI) {
//         *req.uri_mut() = uri;
//     }
//     req.extensions_mut().insert(path);
//     req
// }

pub type T = coral_net::udp::H3;
pub type R = axum::body::Body;
pub type H = coral_net::udp::H3ClientRecv<h3_quinn::RecvStream>;

async fn server(args: &cli::Cli) -> CoralRes<()> {
    let pool = coral_net::client::VecClients::<T, R, H>::default();
    let map_req_fn = move |mut req: hyper::Request<()>| -> hyper::Request<()> {
        println!("---{:?}", req.method());
        println!("---{:?}", req.uri());
        let pool = pool.clone();
        req.extensions_mut().insert(pool);
        if let Some(u) = req.uri().path_and_query() {
            if u.path() == RECV_ENDPOINTS {
                return req;
            }
        }
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
    };
    let addr = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        args.server_param.port,
    );
    let h3_server =
        coral_net::server::ServerBuiler::new(addr, coral_net::tls::server_conf(&args.tls_param)?)
            .set_router(crate::http::app())
            .h3_server(map_req_fn)?;
    Ok(h3_server.run_server().await?)
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-proxy")?;
    if let Err(err) = rt.block_on(server(&args)) {
        error!(e = format!("{:?}", err); "block on server {:?}", args);
    }
    Ok(())
}
