use axum::extract::Request;
use coral_log::tracing::error;
use coral_runtime::tokio;
use hyper::{body::Incoming, service::service_fn};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tower::Service;

use crate::{
    cli,
    error::{CoralRes, Error},
    hand,
};

async fn server(args: cli::Cli) -> CoralRes<()> {
    let app = hand::app();
    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let listen = tokio::net::TcpListener::bind(addr).await?;
    loop {
        let socket = listen.accept().await;
        if let Err(err) = socket {
            error!(e = err.to_string(), "listen accept error");
            continue;
        }
        let (stream, _) = socket.unwrap();
        let tower_serv = app.clone();
        let handle = service_fn(move |request: Request<Incoming>| tower_serv.clone().call(request));
        let io = TokioIo::new(stream);
        if let Err(err) = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
            .serve_connection(io, handle)
            .await
        {
            error!(e = err.to_string(), "http2 builder failed");
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
    let rt = coral_runtime::runtime(args.cpui, args.nums, "coral-server")?;
    if let Err(err) = rt.block_on(server(args)) {
        error!(e = err.to_string(), "block on server error");
    }
    Ok(())
}
