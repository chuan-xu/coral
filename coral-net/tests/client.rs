use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::HeaderValue;
use bytes::Buf;
use bytes::Bytes;
use coral_net::client::Request;
use coral_net::tls::client_conf;
use coral_net::tls::TlsParam;
use coral_runtime::tokio;
use http_body_util::BodyStream;
use http_body_util::Full;
use hyper::HeaderMap;
use hyper::Version;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;

fn get_config() -> rustls::ClientConfig {
    let param = TlsParam {
        tls_ca: Some(String::from("/root/certs/ca")),
        tls_cert: String::from("/root/certs/client.crt"),
        tls_key: String::from("/root/certs/client.key"),
    };
    client_conf(&param).unwrap()
}

use axum::body::BodyDataStream;
use rustls::pki_types;
use tokio_rustls::TlsConnector;
use tokio_stream::StreamExt;

async fn h1_2() -> Result<(), coral_net::error::Error> {
    let tcp_stream = tokio::net::TcpStream::connect("server.test.com:9001").await?;
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(get_config()));
    let domain = "server.test.com".try_into()?;
    let tls_stream = tls_connector.connect(domain, tcp_stream).await?;
    let socket = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(TokioIo::new(tls_stream))
        .await
        .unwrap();
    let (mut send, conn) = socket;
    tokio::spawn(async move {
        if let Err(err) = conn.await {
            println!("http2 client disconnect {:?}", err);
        }
    });
    let req = hyper::Request::builder()
        .version(Version::HTTP_2)
        .method("POST")
        .uri("https://server.test.com:9001/hello")
        .header("name", HeaderValue::from_static("kiljiaden"))
        .body(Full::new(bytes::Bytes::from_static(
            b"hello from tests client",
        )))
        .unwrap();
    let resp = send.send_request(req).await.unwrap();
    let mut body = BodyStream::new(resp.into_body());
    while let Some(data) = body.next().await {
        println!("{:?}", data);
    }
    Ok(())
}

#[test]
fn h1_2_client() {
    let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(h1_2()).unwrap();
}

async fn h3() {
    let tls_config = get_config();
    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config).unwrap(),
    ));

    let mut client_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse().unwrap()).unwrap();
    client_endpoint.set_default_client_config(client_config);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9001);
    let conn = client_endpoint
        .connect(addr, "server.test.com")
        .unwrap()
        .await
        .unwrap();

    let quinn_conn = h3_quinn::Connection::new(conn);

    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await.unwrap();
    coral_runtime::tokio::spawn(async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx))
            .await
            .unwrap();
    });
    let req = hyper::Request::builder()
        .method("POST")
        .uri("https://server.test.com:9001/hello")
        .body(())
        .unwrap();
    let stream = send_request.send_request(req).await.unwrap();
    let (mut tx, mut rx) = stream.split();
    tx.send_data(Bytes::from_static(b"hello from tests h3 client"))
        .await
        .unwrap();
    let mut h = HeaderMap::new();
    h.insert("age", HeaderValue::from_static("18"));
    tx.send_trailers(h).await.unwrap();
    tx.finish().await.unwrap();
    let res = rx.recv_response().await.unwrap();
    println!("{:?}", res.headers());
    println!("{:?}", res.version());
    println!("{:?}", res.status());
    while let Some(data) = rx.recv_data().await.unwrap() {
        println!("{:?}", std::str::from_utf8(data.chunk()));
    }
    let trailers = rx.recv_trailers().await;
    println!("trailers: {:?}", trailers);
}

#[test]
fn h3_client() {
    let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(h3());
}

async fn coral_h1_create() {
    let stream = tokio::net::TcpStream::connect("127.0.0.1:9001")
        .await
        .unwrap();
    let domian = pki_types::ServerName::try_from("server.test.com").unwrap();
    let connector = TlsConnector::from(Arc::new(get_config()));
    let tls_socket = connector.connect(domian, stream).await.unwrap();
    let builder = hyper::client::conn::http1::Builder::new();
    let mut h1: coral_net::tcp::H1<BodyDataStream> =
        coral_net::tcp::H1::new(tls_socket, builder).await.unwrap();
    let abody = axum::body::Body::from("hello");
    let bs = abody.into_data_stream();
    let request = hyper::Request::builder().body(bs).unwrap();
    h1.send(request).await.unwrap();
}

#[test]
fn coral_h1_client() {
    let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(coral_h1_create());
}

async fn coral_h2_create() {
    let stream = tokio::net::TcpStream::connect("127.0.0.1:9001")
        .await
        .unwrap();
    let domian = pki_types::ServerName::try_from("server.test.com").unwrap();
    let connector = TlsConnector::from(Arc::new(get_config()));
    let tls_socket = connector.connect(domian, stream).await.unwrap();
    let builder = hyper::client::conn::http2::Builder::new(TokioExecutor::new());
    let mut h2: coral_net::tcp::H2<BodyDataStream> =
        coral_net::tcp::H2::new(tls_socket, builder).await.unwrap();
    let abody = axum::body::Body::from("hello");
    let bs = abody.into_data_stream();
    let request = hyper::Request::builder().body(bs).unwrap();
    h2.send(request).await.unwrap();
    let _h2_clone = h2.clone();
}

#[test]
fn coral_h2_client() {
    let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(coral_h2_create());
}
