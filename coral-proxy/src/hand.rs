use axum::{extract::Request, http::uri::PathAndQuery};

use crate::io::PxyChan;

pub async fn proxy(req: Request) -> hyper::Response<hyper::body::Incoming> {
    let uri = req.extensions().get::<PathAndQuery>().unwrap().clone();
    let mut pxy_ch = req.extensions().get::<PxyChan>().unwrap().clone();
    let headers = req.headers().clone();
    let body = req.into_body().into_data_stream();
    let mut pxy_builder = hyper::Request::builder().method("POST").uri(uri);
    let pxy_headers = pxy_builder.headers_mut().unwrap();
    *pxy_headers = headers;
    let pxy_req = pxy_builder.body(body).unwrap();
    let rsp = pxy_ch.get_sender().send_request(pxy_req).await.unwrap();
    rsp
}
