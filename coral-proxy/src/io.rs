use axum::{routing::post, Router};
use coral_runtime::tokio;
// use futures::StreamExt;
use coral_log::{
    error,
    tracing::{self, span},
    Level,
};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::cell::RefCell;
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::{
    cli,
    error::{CoralRes, Error},
    hand::{self, PxyChan},
    tls,
};

fn service_fn(
    mut req: hyper::Request<Incoming>,
    pxy_chan: Option<PxyChan>,
    mut tower_service: Router,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    if let Some(v) = req.headers().get("X-Trace-Id") {
        let trace_id = v.to_str().unwrap();
        span!(Level::INFO, "trace_id", val = trace_id);
    } else {
        // TODO
    }
    let uri = req.uri();
    let path = uri.path_and_query().unwrap().to_owned();
    let nuri = match uri.scheme_str() {
        Some(scheme_str) => {
            let mut scheme = scheme_str.to_string();
            scheme += "://";
            scheme += uri.authority().unwrap().as_str();
            hyper::Uri::try_from(scheme).unwrap()
        }
        None => hyper::Uri::from_static("/"),
    };
    *(req.uri_mut()) = nuri;
    req.extensions_mut().insert(path);
    req.extensions_mut().insert(pxy_chan);
    tower_service.call(req)
}

async fn hand_stream(
    tls_accept: TlsAcceptor,
    cnx: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    tower_service: Router,
    pxy_chan: Option<PxyChan>,
) {
    match tls_accept.accept(cnx).await {
        Ok(stream) => {
            // TODO temporary use TokioIo
            let stream = TokioIo::new(stream);
            let hyper_service = hyper::service::service_fn(move |req: hyper::Request<Incoming>| {
                service_fn(req, pxy_chan.clone(), tower_service.clone())
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

async fn tcp_accept(
    app: &Router,
    tls_acceptor: &TlsAcceptor,
    tcp_listener: &tokio::net::TcpListener,
    pxy_chan: &Vec<PxyChan>,
) -> CoralRes<()> {
    let tower_service = app.clone();
    let tls_accept = tls_acceptor.clone();
    let ch = match pxy_chan.iter().min_by_key(|v| v.ref_count()) {
        Some(c) => Some(c.clone()),
        None => None,
    };
    match tcp_listener.accept().await {
        Ok((cnx, addr)) => {
            tokio::spawn(hand_stream(tls_accept, cnx, addr, tower_service, ch));
        }
        Err(_) => todo!(),
    }
    Ok(())
}

// #[coral_log::instrument]
async fn server(args: cli::Cli) -> CoralRes<()> {
    let conf = tls::server_conf(&args)?;
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(conf);
    let bind = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let tcp_listener = tokio::net::TcpListener::bind(bind).await?;
    let app = Router::new().route("/", post(hand::proxy));
    // let mut pxy_chans: Vec<_> = futures::stream::iter(&args.addresses)
    //     .then(|item| PxyChan::new(item))
    //     .collect()
    //     .await;
    // pxy_chans.retain(|item| item.is_ok());
    let mut pxy_chan = Vec::new();
    for addr in &args.addresses {
        match PxyChan::new(addr).await {
            Ok(ch) => pxy_chan.push(ch),
            Err(e) => error!(error = e.to_string(), "failed to new proxy channel"),
        }
    }

    futures::pin_mut!(tcp_listener);
    loop {
        if let Err(e) = tcp_accept(&app, &tls_acceptor, &tcp_listener, &pxy_chan).await {
            error!(error = e.to_string(), "failed to tcp accept");
        }
    }
}

fn before_fn(debug: bool, writer: coral_log::NonBlocking) -> impl Fn() {
    move || {
        std::thread_local! {
            static SUBSCRIBER_GUARD: RefCell<Option<coral_log::DefaultGuard>> = RefCell::new(None);
        }
        let guard = coral_log::subscriber(debug, writer.clone());
        if let Err(e) = SUBSCRIBER_GUARD.try_with(|g| {
            *g.borrow_mut() = Some(guard);
        }) {
            eprintln!("failed to set SUBSCRIBER_GUARD with {:?}", e);
        }
    }
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let log_handler = if args.debug {
        coral_log::WriterHandler::stdout()
    } else {
        let dir = args.log_dir.as_ref().ok_or(Error::MissingLogDir)?;
        coral_log::WriterHandler::fileout(dir, "coral-proxy", args.get_rotation()?)
    };
    let _guard = coral_log::subscriber(args.debug, log_handler.get_writer());
    let rt = coral_runtime::runtime(
        args.cpui,
        args.nums,
        "coral-proxy",
        before_fn(args.debug, log_handler.get_writer()),
    )?;
    rt.block_on(server(args))?;
    Ok(())
}
