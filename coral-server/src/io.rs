use std::net::Ipv4Addr;
use std::net::SocketAddr;

use bytes::Bytes;
use coral_runtime::tokio;
use log::error;

use crate::cli;
use crate::error::CoralRes;

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
