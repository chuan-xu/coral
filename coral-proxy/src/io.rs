use axum::routing::post;
use axum::Router;
use coral_runtime::tokio;
use hyper::body::Incoming;
use hyper::header::CONNECTION;
use hyper::header::SEC_WEBSOCKET_KEY;
use hyper::header::UPGRADE;
use hyper::Method;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use log::info;
use tokio_rustls::TlsAcceptor;

use crate::cli;
use crate::error::CoralRes;
use crate::http::http_reset;
use crate::http::PxyPool;
use crate::http::{self};
use crate::tls;
use crate::util;
use crate::ws;
use crate::ws::websocket_reset;

fn handle_request(
    req: hyper::Request<Incoming>,
    pxy_pool: PxyPool,
    router: Router,
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
            .map(|v| v.to_uppercase() == "websocket")
            .unwrap_or(false)
        && headers.get(SEC_WEBSOCKET_KEY).is_some()
        && req.method() == Method::GET
    {
        websocket_reset(req, router)
    } else {
        http_reset(req, pxy_pool, router)
    }
}

async fn hand_stream(
    tls_accept: TlsAcceptor,
    cnx: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    tower_service: Router,
    pxy_pool: PxyPool,
) {
    info!("new connection: {}", addr);
    match tls_accept.accept(cnx).await {
        Ok(stream) => {
            let stream = TokioIo::new(stream);
            let hyper_service = hyper::service::service_fn(|req: hyper::Request<Incoming>| {
                // handle_request(&req);
                handle_request(req, pxy_pool.clone(), tower_service.clone())
            });
            let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(stream, hyper_service)
                .await;
            if let Err(err) = ret {
                error!("error serving connection from {}: {}", addr, err);
            }
        }
        Err(e) => {
            error!("tls accept error {}", e);
        }
    }
}

async fn server(args: cli::Cli) -> CoralRes<()> {
    args.log_param.set_traces();
    let conf = tls::server_conf(&args)?;
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(conf);
    let bind = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let tcp_listener = tokio::net::TcpListener::bind(bind).await?;
    let app = Router::new()
        .route(util::HTTP_RESET_URI, post(http::proxy))
        // .route(util::WS_RESET_URI, post(ws::proxy))
        .layer(coral_util::tow::TraceLayer::default());
    let pxy_pool = PxyPool::build(&args.addresses).await?;

    futures::pin_mut!(tcp_listener);
    loop {
        match tcp_listener.accept().await {
            Ok((cnx, addr)) => {
                tokio::spawn(hand_stream(
                    tls_acceptor.clone(),
                    cnx,
                    addr,
                    app.clone(),
                    pxy_pool.clone(),
                ));
            }
            Err(err) => {
                let e_str = err.to_string();
                error!(e = e_str.as_str(); "failed to tcp accept");
            }
        }
    }
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-proxy")?;
    if let Err(err) = rt.block_on(server(args)) {
        let e_str = err.to_string();
        error!(e = e_str.as_str(); "block on server error");
    }
    Ok(())
}
