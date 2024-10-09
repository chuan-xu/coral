use bytes::Bytes;
use coral_runtime::spawn;
use log::error;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::cli;
use crate::cli::Conf;
use crate::error::CoralRes;
use crate::hand;

async fn report<F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static>(
    h3_server: coral_net::server::H3Server<F>,
    service_address: &str,
    authority: String,
) -> CoralRes<()> {
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

async fn server(conf: Conf, app: axum::Router) -> CoralRes<()> {
    conf.log_conf.set_traces();
    let addr_h2 = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        conf.h2.server_conf.port,
    );
    crate::hand::H3_PORT.set(conf.h3.server_conf.port).unwrap();
    spawn(
        coral_net::server::ServerBuiler::new(addr_h2, conf.h2.tls_conf.server_conf()?)
            .set_router(app.clone())
            .h2_server(Some(coral_net::hand::redirect_h2)),
    );

    let addr_h3 = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        conf.h3.server_conf.port,
    );
    let mut transport_config = quinn_proto::TransportConfig::default();
    transport_config.max_idle_timeout(Some(quinn_proto::VarInt::from_u32(3600000).into()));
    let h3_server = coral_net::server::ServerBuiler::new(addr_h3, conf.h3.tls_conf.server_conf()?)
        .set_router(app)
        // .set_client_tls()
        .h3_server(Some(Arc::new(transport_config)), |req| req)?;

    if conf.h3.server_conf.domain.is_some() && conf.h3.service_address.is_some() {
        let authority = format!(
            "{}:{}",
            conf.h3.server_conf.domain.as_ref().unwrap(),
            conf.h3.server_conf.port
        );
        report(
            h3_server.clone(),
            conf.h3.service_address.as_ref().unwrap(),
            authority,
        )
        .await?;
    }
    Ok(h3_server.run_server().await?)
}

pub fn run() -> CoralRes<()> {
    let conf = cli::Cli::init()?;
    let rt = conf.rt_conf.runtime("coral_server")?;
    let app = hand::app(&conf);
    if let Err(err) = rt.block_on(server(conf, app)) {
        error!(e = format!("{:?}", err); "block on server");
    }
    Ok(())
}
