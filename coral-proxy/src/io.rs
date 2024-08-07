use axum::{
    http::{uri::PathAndQuery, Uri},
    routing::post,
    Router,
};
use coral_log::error;
use coral_runtime::tokio;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::cell::RefCell;
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::{
    cli,
    error::{CoralRes, Error},
    hand::{self, PxyPool},
    tls,
};

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
            error!(
                e = err.to_string(),
                scheme = scheme_str,
                authority = authority,
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

async fn tcp_accept(
    app: &Router,
    tls_acceptor: &TlsAcceptor,
    tcp_listener: &tokio::net::TcpListener,
    pxy_pool: &PxyPool,
) -> CoralRes<()> {
    let tower_service = app.clone();
    let tls_accept = tls_acceptor.clone();
    match tcp_listener.accept().await {
        Ok((cnx, addr)) => {
            tokio::spawn(hand_stream(
                tls_accept,
                cnx,
                addr,
                tower_service,
                pxy_pool.clone(),
            ));
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

    // TODO del
    // let mut pxy_chan = Vec::new();
    // for addr in &args.addresses {
    //     match PxyChan::new(addr).await {
    //         Ok(ch) => pxy_chan.push(ch),
    //         Err(e) => error!(error = e.to_string(), "failed to new proxy channel"),
    //     }
    // }

    let pxy_pool = PxyPool::build(&args.addresses).await?;

    futures::pin_mut!(tcp_listener);
    loop {
        if let Err(err) = tcp_accept(&app, &tls_acceptor, &tcp_listener, &pxy_pool).await {
            error!(e = err.to_string(), "failed to tcp accept");
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
