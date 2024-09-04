use std::net::Ipv4Addr;
use std::net::SocketAddr;

use axum::extract::Request;
use bytes::Bytes;
use coral_runtime::tokio;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use log::info;
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

// async fn notify(args: &cli::Cli) -> CoralRes<()> {
//     if let Some(addr) = args.comm_param.cache_addr.as_ref() {
//         let mut client = coral_util::db::cache::MiniRedis::new(addr).await?;
//         let local = SocketAddr::new(local_ip_address::local_ip()?, args.port).to_string();
//         let endpoints = match client.get(coral_util::consts::REDIS_KEY_DISCOVER).await? {
//             Some(data) => {
//                 let mut ends = data.split(|k| *k == 44).collect::<Vec<&[u8]>>();
//                 if !check_ends_repeat(&ends, local.as_bytes()) {
//                     ends.push(local.as_bytes());
//                 }
//                 bytes::Bytes::from(ends.join(&44))
//             }
//             None => bytes::Bytes::from(local),
//         };

//         client
//             .set(coral_util::consts::REDIS_KEY_DISCOVER, endpoints)
//             .await?;
//         client
//             .publish(coral_util::consts::REDIS_KEY_NOTIFY, bytes::Bytes::from(""))
//             .await?;
//     }
//     Ok(())
// }

// async fn server_http2(args: cli::Cli) -> CoralRes<()> {
//     args.log_param.set_traces();
//     notify(&args).await?;
//     let app = hand::app();
//     let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
//     let listen = tokio::net::TcpListener::bind(addr).await?;
//     loop {
//         let socket = listen.accept().await;
//         if let Err(err) = socket {
//             error!(e = format!("{:?}", err); "listen accept error");
//             continue;
//         }
//         let tower_serv = app.clone();
//         tokio::spawn(async move {
//             let (stream, _) = socket.unwrap();
//             let handle =
//                 service_fn(move |request: Request<Incoming>| tower_serv.clone().call(request));
//             let io = TokioIo::new(stream);
//             if let Err(err) = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
//                 .serve_connection(io, handle)
//                 .await
//             {
//                 error!(e = format!("{:?}", err); "http2 builder failed");
//             }
//         });
//     }
// }

async fn report<F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static>(
    h3_server: coral_net::server::H3Server<F>,
    service_address: &str,
    authority: String,
) -> CoralRes<()> {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let (addr, domain) = coral_net::client::lookup_host(service_address).await?;
    let mut sender = h3_server.create_h3_client(addr, &domain, true).await?;
    let req = hyper::Request::builder()
        .method("POST")
        .uri(service_address)
        .body(())
        .map_err(|e| crate::error::Error::CoralNetErr(coral_net::error::Error::HttpInner(e)))?;
    let map_h3_err = |e| crate::error::Error::CoralNetErr(coral_net::error::Error::H3Err(e));
    let mut stream = sender.send_request(req).await.map_err(map_h3_err)?;
    stream.send_data(Bytes::from(authority)).await?;
    stream.finish().await.map_err(map_h3_err)?;
    let rsp = stream.recv_response().await.map_err(map_h3_err)?;
    if !rsp.status().is_success() {
        error!(
            "failed to report local to service with status: {}",
            rsp.status()
        );
    }
    Ok(())
}

async fn server(args: &cli::Cli) -> CoralRes<()> {
    let addr = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        args.server_param.port,
    );
    let h3_server =
        coral_net::server::ServerBuiler::new(addr, coral_net::tls::server_conf(&args.tls_param)?)
            .set_router(crate::hand::app())
            .set_client_tls(coral_net::tls::client_conf(&args.tls_param)?)
            .h3_server(|req| req)?;

    let authority = format!("{}:{}", args.domain, args.server_param.port);
    report(h3_server.clone(), &args.service_address, authority).await?;
    Ok(h3_server.run_server().await?)
}

pub fn run() -> CoralRes<()> {
    let args = cli::Cli::init()?;
    let rt = coral_runtime::runtime(&args.runtime_param, "coral-server")?;
    if let Err(err) = rt.block_on(server(&args)) {
        error!(e = format!("{:?}", err); "block on server {:?}", args);
    }
    Ok(())
}
