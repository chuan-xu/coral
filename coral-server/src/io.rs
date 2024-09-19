use bytes::Bytes;
use coral_runtime::tokio;
use log::error;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::cli;
use crate::error::CoralRes;

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

async fn server(args: &cli::Cli) -> CoralRes<()> {
    args.log_param.set_traces();
    let tls_conf = coral_net::tls::server_conf(&args.tls_param)?;
    let addr_h2 = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        args.server_param.port,
    );
    crate::hand::H3_PORT
        .set(args.server_param.port + 1)
        .unwrap();
    tokio::spawn(
        coral_net::server::ServerBuiler::new(addr_h2, tls_conf.clone())
            .set_router(crate::hand::upgrade_app())
            .h2_server(Some(coral_net::hand::redirect_h2)),
    );

    let addr_h3 = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        args.server_param.port + 1,
    );
    let mut transport_config = quinn_proto::TransportConfig::default();
    transport_config.max_idle_timeout(Some(quinn_proto::VarInt::from_u32(3600000).into()));
    let h3_server = coral_net::server::ServerBuiler::new(addr_h3, tls_conf)
        .set_router(crate::hand::app())
        .set_client_tls(coral_net::tls::client_conf(&args.tls_param)?)
        .h3_server(Some(Arc::new(transport_config)), |req| req)?;

    if args.domain.is_some() && args.service_address.is_some() {
        let authority = format!(
            "{}:{}",
            args.domain.as_ref().unwrap(),
            args.server_param.port + 1
        );
        report(
            h3_server.clone(),
            args.service_address.as_ref().unwrap(),
            authority,
        )
        .await?;
    }
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
