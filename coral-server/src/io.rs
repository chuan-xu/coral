use std::cell::RefCell;

use axum::extract::Request;
use coral_runtime::tokio;
use hyper::{body::Incoming, service::service_fn};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tower::Service;

use crate::{
    cli,
    error::{CoralRes, Error},
    hand,
};

async fn server(args: cli::Cli) {
    let app = hand::app();
    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let listen = tokio::net::TcpListener::bind(addr).await.unwrap();
    loop {
        let tower_serv = app.clone();
        let (stream, _) = listen.accept().await.unwrap();
        println!("conn");
        let handle = service_fn(move |request: Request<Incoming>| tower_serv.clone().call(request));
        let io = TokioIo::new(stream);
        hyper::server::conn::http2::Builder::new(TokioExecutor::new())
            .serve_connection(io, handle)
            .await
            .unwrap();
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
        "coral-server",
        before_fn(args.debug, log_handler.get_writer()),
    )?;
    rt.block_on(server(args));
    Ok(())
}
