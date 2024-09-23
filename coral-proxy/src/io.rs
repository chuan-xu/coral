use std::convert::Infallible;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::future::RouteFuture;
use coral_runtime::tokio;
use hyper::body::Incoming;
use log::error;

use crate::cli;
use crate::cli::Conf;
use crate::error::CoralRes;
use crate::http::RECV_ENDPOINTS;

pub type T = coral_net::udp::H3;
pub type R = axum::body::Body;
pub type H = coral_net::udp::H3ClientRecv<h3_quinn::RecvStream>;
pub type Pool = coral_net::client::VecClients<T, R, H>;

fn map_req_h3(mut req: hyper::Request<()>, pool: Pool) -> hyper::Request<()> {
    req.extensions_mut().insert(pool);
    if let Some(u) = req.uri().path_and_query() {
        if u.path() == RECV_ENDPOINTS {
            return req;
        }
    }
    coral_net::hand::redirect_req(&mut req, coral_net::hand::HTTP_RESET_URI);
    req
}

async fn server(conf: Conf) -> CoralRes<()> {
    conf.log_conf.set_traces();
    let pool = coral_net::client::VecClients::<T, R, H>::default();
    let poolc = pool.clone();
    let map_req_fn_h3 =
        move |req: hyper::Request<()>| -> hyper::Request<()> { map_req_h3(req, poolc.clone()) };
    let addr_h2 = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        conf.h2.server_conf.port,
    );
    let addr_h3 = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        conf.h3.server_conf.port,
    );
    let mut transport_config = quinn_proto::TransportConfig::default();
    transport_config.max_idle_timeout(Some(quinn_proto::VarInt::from_u32(3600000).into()));
    let h3_server = coral_net::server::ServerBuiler::new(addr_h3, conf.h3.tls_conf.server_conf()?)
        .set_router(crate::http::app_h3())
        .h3_server(Some(Arc::new(transport_config)), map_req_fn_h3)?;
    tokio::spawn(async move {
        if let Err(err) = h3_server.run_server().await {
            error!(e = format!("{:?}", err); "failed to run h3 server");
        }
    });
    let map_req_fn_h2 =
        move |mut req: hyper::Request<Incoming>, router| -> RouteFuture<Infallible> {
            req.extensions_mut().insert(pool.clone());
            coral_net::hand::redirect_h2(req, router)
        };
    Ok(
        coral_net::server::ServerBuiler::new(addr_h2, conf.h3.tls_conf.server_conf()?)
            .set_router(crate::http::app_h2())
            .h2_server(Some(map_req_fn_h2))
            .await?,
    )
}

pub fn run() -> CoralRes<()> {
    let conf = cli::Cli::init()?;
    let rt = conf.rt_conf.runtime("coral-proxy")?;
    if let Err(err) = rt.block_on(server(conf)) {
        error!(e = format!("{:?}", err); "block on server");
    }
    Ok(())
}
