use axum::extract::Request;
use axum::http::uri::PathAndQuery;
use axum::routing::post;
use coral_macro::trace_error;
use coral_macro::trace_info;
use coral_net::client::Request as CoralNetReq;
use coral_runtime::tokio;
use coral_util::tow::add_header_span_id;

use crate::error::CoralRes;
use crate::error::Error;

// #[axum::debug_handler]
async fn proxy(
    req: Request,
) -> CoralRes<
    hyper::Response<
        impl hyper::body::Body<Data = bytes::Bytes, Error = Box<dyn std::error::Error + Send + Sync>>,
    >,
> {
    // get origin uri path
    let uri = req
        .extensions()
        .get::<PathAndQuery>()
        .ok_or_else(|| {
            trace_error!("PathAndQuery is none");
            Error::NoneOption("PathAndQuery ")
        })?
        .clone();
    let headers = req.headers().clone();
    let method = req.method().clone();

    // TODO: tmp use unwrap
    let pool = req
        .extensions()
        .get::<coral_net::client::VecClients<crate::io::T, crate::io::R, crate::io::H>>()
        .unwrap()
        .clone();

    let body = req.into_body().into_data_stream();
    let mut trans_builder = hyper::Request::builder()
        .method(method)
        .uri(uri)
        .version(hyper::Version::HTTP_3);
    let trans_headers = trans_builder.headers_mut().ok_or_else(|| {
        trace_error!("faile to get trans header");
        Error::NoneOption("trans header")
    })?;
    *trans_headers = headers;
    add_header_span_id(trans_headers);

    let trans_req = trans_builder.body(body).map_err(|err| {
        trace_error!(e = format!("{:?}", err); "failed to build trans body");
        err
    })?;

    let (mut sender, _guard) = pool.load_balance().await?.unwrap();
    let rsp = sender.send(trans_req).await?;
    Ok(rsp)
}

async fn recv_endpoints(req: Request) -> CoralRes<()> {
    trace_info!("new endpoint conn");
    let pool = req
        .extensions()
        .get::<coral_net::client::VecClients<crate::io::T, crate::io::R, crate::io::H>>()
        .unwrap()
        .clone();
    let conn = req.extensions().get::<quinn::Connection>().unwrap().clone();
    tokio::spawn(async move {
        // wait for the original connection to disconnect
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        match coral_net::udp::H3::new_by_connection(conn, true).await {
            Ok(conn) => {
                pool.add(conn).await;
            }
            Err(err) => {
                trace_error!(e = format!("{:?}", err); "failed to establish connection")
            }
        }
    });
    Ok(())
}

pub static RESET_URI: &'static str = "/reset";
pub static RECV_ENDPOINTS: &'static str = "/coral-proxy-endpoints";

pub fn app() -> axum::Router {
    let router: axum::Router = axum::Router::new()
        .route(RESET_URI, post(proxy))
        .route(RECV_ENDPOINTS, post(recv_endpoints));
    router
}
