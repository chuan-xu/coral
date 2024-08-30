use std::convert::Infallible;

use axum::http::HeaderValue;
use axum::routing::post;
use bytes::Buf;
use bytes::Bytes;
use coral_net::tls::server_conf;
use coral_net::tls::TlsParam;
fn get_config() -> rustls::ServerConfig {
    let param = TlsParam {
        tls_ca: Some(String::from("/root/certs/ca")),
        tls_cert: String::from("/root/certs/server.crt"),
        tls_key: String::from("/root/certs/server.key"),
    };
    server_conf(&param).unwrap()
}

// async fn hello<T: hyper::body::Body + Unpin>(req: hyper::Request<T>) -> &'static str {
async fn hello<T: hyper::body::Body + Unpin>(
    req: hyper::Request<T>,
) -> hyper::Response<StreamBody<ReceiverStream<Result<Frame<Bytes>, Infallible>>>> {
    println!("=====> in server");
    let version = req.version();
    let mut stream = BodyStream::new(req.into_body());
    while let Some(data) = stream.next().await {
        if let Ok(data) = data {
            if data.is_data() {
                if let Ok(d) = data.into_data() {
                    println!("{:?}", std::str::from_utf8(d.chunk()).unwrap());
                }
            } else if data.is_trailers() {
                if let Ok(d) = data.into_trailers() {
                    println!("{:?}", d);
                }
            }
        }
    }
    type Data = Result<Frame<Bytes>, Infallible>;
    let (tx, rx) = mpsc::channel::<Data>(3);
    coral_runtime::tokio::spawn(async move {
        let mut h = HeaderMap::new();
        h.insert("age", HeaderValue::from_static("19"));
        let f1 = Frame::data(Bytes::from_static(b"hello from tests server"));

        tx.send(Ok(f1)).await.unwrap();

        // headers based off expensive operation
        tx.send(Ok(Frame::trailers(h))).await.unwrap();
    });
    let stream = ReceiverStream::new(rx);

    let body = StreamBody::new(stream);
    let resp = hyper::Response::builder()
        .version(version)
        // .header(TRAILER, "age")
        .body(body)
        .unwrap();
    // let mut res_b = BodyStream::new(resp.into_body());
    // while let Some(data) = res_b.next().await {
    //     let data = data.unwrap();
    //     println!("{:?}", data);
    // }
    // "hello from server"
    resp
}

use coral_net::server::HttpServ;
use coral_runtime::tokio::sync::mpsc;
use http_body::Frame;
use http_body_util::BodyExt;
use http_body_util::BodyStream;
use http_body_util::Full;
use http_body_util::StreamBody;
use hyper::header::TRAILER;
use hyper::HeaderMap;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
async fn h1_2() {
    let h = coral_net::server::Builder::default()
        .tls_config(get_config())
        .http1_or_2(true)
        .address(String::from("0.0.0.0:9001"))
        .http1_2();

    let router: axum::Router = axum::Router::new().route("/hello", post(hello));
    h.run(router).await.unwrap();
}

#[test]
fn h1_2_server() {
    let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(h1_2());
}

async fn h3() {
    println!("in h3");
    let h = coral_net::server::Builder::default()
        .tls_config(get_config())
        .http3();
    let router: axum::Router = axum::Router::new().route("/hello", post(hello));
    h.run(router).await.unwrap();
}

#[test]
fn h3_server() {
    let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(h3());
}
