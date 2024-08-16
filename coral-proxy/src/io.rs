use axum::http::uri::PathAndQuery;
use axum::http::Uri;
use axum::routing::post;
use axum::Router;
use coral_runtime::tokio;
use hyper::body::Incoming;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::cli;
use crate::error::CoralRes;
use crate::error::Error;
use crate::hand::PxyPool;
use crate::hand::{self};
use crate::tls;

fn parse_url(uri: &Uri) -> CoralRes<(&PathAndQuery, Uri)> {
    let path = uri.path_and_query().ok_or_else(|| {
        error!("uri.path_and_query is none");
        Error::NoneOption("uri.path_and_query")
    })?;
    let authority = uri
        .authority()
        .ok_or_else(|| {
            error!("uri.authority is none");
            Error::NoneOption("uri.authority")
        })?
        .as_str();
    if let Some(scheme_str) = uri.scheme_str() {
        let mut scheme = scheme_str.to_string();
        scheme += "://";
        scheme += authority;
        let nuri = hyper::Uri::try_from(scheme).map_err(|err| {
            let e_str = err.to_string();
            error!(
                e = e_str.as_str(),
                scheme = scheme_str,
                authority = authority;
                "failed to parse scheme"
            );
            err
        })?;
        Ok((path, nuri))
    } else {
        Ok((path, hyper::Uri::from_static("/")))
    }
}

fn service_fn(
    mut req: hyper::Request<Incoming>,
    pxy_pool: PxyPool,
    mut tower_service: Router,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    let uri = req.uri();
    let (path, nuri) = parse_url(uri).unwrap();
    let path = path.to_owned();
    *(req.uri_mut()) = nuri;
    req.extensions_mut().insert(path);
    req.extensions_mut().insert(pxy_pool);
    tower_service.call(req)
}

async fn hand_stream(
    tls_accept: TlsAcceptor,
    cnx: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    tower_service: Router,
    pxy_pool: PxyPool,
) {
    match tls_accept.accept(cnx).await {
        Ok(stream) => {
            let stream = TokioIo::new(stream);
            let hyper_service = hyper::service::service_fn(|req: hyper::Request<Incoming>| {
                service_fn(req, pxy_pool.clone(), tower_service.clone())
            });
            let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(stream, hyper_service)
                .await;
            if let Err(err) = ret {
                println!("error serving connection from {}: {}", addr, err);
            }
        }
        Err(_) => {
            // TODO
        }
    }
}

async fn server(args: cli::Cli) -> CoralRes<()> {
    let conf = tls::server_conf(&args)?;
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(conf);
    let bind = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let tcp_listener = tokio::net::TcpListener::bind(bind).await?;
    let app = Router::new()
        .route("/", post(hand::proxy))
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
    rt.block_on(server(args))?;
    Ok(())
}
