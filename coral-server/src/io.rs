use std::net::SocketAddr;

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

fn check_ends_repeat(endpoints: &Vec<&[u8]>, local: &[u8]) -> bool {
    let mut repeated = false;
    for p in endpoints.iter() {
        if **p == *local {
            repeated = true;
            break;
        }
    }
    repeated
}

async fn notify(args: &cli::Cli) -> CoralRes<()> {
    if let Some(addr) = args.comm_param.cache_addr.as_ref() {
        let mut client = coral_util::db::cache::MiniRedis::new(addr).await?;
        let local = SocketAddr::new(local_ip_address::local_ip()?, args.port).to_string();
        let endpoints = match client.get(coral_util::consts::REDIS_KEY_DISCOVER).await? {
            Some(data) => {
                let mut ends = data.split(|k| *k == 44).collect::<Vec<&[u8]>>();
                if !check_ends_repeat(&ends, local.as_bytes()) {
                    ends.push(local.as_bytes());
                }
                bytes::Bytes::from(ends.join(&44))
            }
            None => bytes::Bytes::from(local),
        };

        client
            .set(coral_util::consts::REDIS_KEY_DISCOVER, endpoints)
            .await?;
        client
            .publish(coral_util::consts::REDIS_KEY_NOTIFY, bytes::Bytes::from(""))
            .await?;
    }
    Ok(())
}

async fn server_http2(args: cli::Cli) -> CoralRes<()> {
    args.log_param.set_traces();
    notify(&args).await?;
    let app = hand::app();
    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let listen = tokio::net::TcpListener::bind(addr).await?;
    loop {
        let socket = listen.accept().await;
        if let Err(err) = socket {
            error!(e = err.to_string(); "listen accept error");
            continue;
        }
        let tower_serv = app.clone();
        tokio::spawn(async move {
            let (stream, _) = socket.unwrap();
            let handle =
                service_fn(move |request: Request<Incoming>| tower_serv.clone().call(request));
            let io = TokioIo::new(stream);
            if let Err(err) = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                .serve_connection(io, handle)
                .await
            {
                error!(e = err.to_string(); "http2 builder failed");
            }
        });
    }
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-server")?;
    if let Err(err) = rt.block_on(server_http2(args)) {
        error!(e = err.to_string(); "block on server http2 error");
    }
    Ok(())
}
