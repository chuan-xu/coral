use axum::extract::Request;
use axum::http::uri::PathAndQuery;
use axum::routing::post;
use coral_macro::trace_error;
use coral_macro::trace_info;
use coral_net::client::Request as CoralNetReq;
use coral_net::midware::add_header_span_id;
use http_body_util::BodyExt;

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
    let path_query = req
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

    // let uri = hyper::Uri::builder()
    //     .scheme("https")
    //     .authority(req.uri().authority().unwrap().clone())
    //     .path_and_query(path_query)
    //     .build()?;

    let body = req.into_body().into_data_stream();
    let mut trans_builder = hyper::Request::builder()
        .method(method)
        .uri(path_query)
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
    let sender = req
        .extensions()
        .get::<h3::client::SendRequest<h3_quinn::OpenStreams, bytes::Bytes>>()
        .unwrap()
        .clone();
    let domain_byte = req.into_body().collect().await.unwrap().to_bytes();
    let domain = std::str::from_utf8(&domain_byte).unwrap().to_owned();
    pool.add(coral_net::udp::H3::new_with_sender(sender, domain))
        .await;
    Ok(())
}

pub static RESET_URI: &'static str = "/reset";
pub static RECV_ENDPOINTS: &'static str = "/coral-proxy-endpoints";

pub fn app_h3() -> axum::Router {
    let router: axum::Router = axum::Router::new()
        .route(RESET_URI, post(proxy))
        .route(RECV_ENDPOINTS, post(recv_endpoints));
    router
}

pub fn app_h2() -> axum::Router {
    let router: axum::Router = axum::Router::new().route(RESET_URI, post(proxy));
    router
}
