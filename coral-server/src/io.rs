use axum::extract::Request;
use coral_runtime::tokio;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use tower::Service;

use crate::cli;
use crate::error::CoralRes;
use crate::hand;

async fn server(args: cli::Cli) -> CoralRes<()> {
    let app = hand::app();
    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let listen = tokio::net::TcpListener::bind(addr).await?;
    loop {
        let socket = listen.accept().await;
        if let Err(err) = socket {
            let e_str = err.to_string();
            error!(e = e_str.as_str(); "listen accept error");
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
            let e_str = err.to_string();
            error!(e = e_str.as_str(); "http2 builder failed");
        }
    }
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-server")?;
    if let Err(err) = rt.block_on(server(args)) {
        let e_str = err.to_string();
        error!(e = e_str.as_str(); "block on server error");
    }
    Ok(())
}
