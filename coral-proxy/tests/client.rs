use std::io::Write;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;
use coral_runtime::tokio;
use futures::future::join_all;
use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("coral net module error")]
    CoralNetErr(#[from] coral_net::error::Error),

    #[error("hyper http inner error")]
    HttpInner(#[from] hyper::http::Error),

    #[error("http invalid uri")]
    HttpUri(#[from] hyper::http::uri::InvalidUri),

    #[error("h3 error")]
    H3Err(#[from] h3::error::Error),

    #[error("resp in not ok")]
    StatusErr,

    #[error("Io Error")]
    IoErr(#[from] std::io::Error),

    #[error("invalid data")]
    InvalidData,
}

async fn handle(conf: quinn::ClientConfig, addr: SocketAddr) -> Result<(), Error> {
    let mut endpoints = quinn::Endpoint::client("[::]:0".parse().unwrap()).unwrap();
    endpoints.set_default_client_config(conf);
    let conn = endpoints
        .connect(addr, "tx.coral.com")
        // .connect(addr, "server.test.com")
        .unwrap()
        .await
        .unwrap();
    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await.unwrap();
    let drive = async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx))
            .await
            .unwrap();
    };
    let request = async move {
        let req = hyper::http::Request::builder()
            .method("POST")
            // .uri("https://server.test.com:9001/benchmark")
            .uri("https://tx.coral.com:9001/benchmark")
            .header(hyper::header::CONTENT_LENGTH, "36")
            // .version(hyper::Version::HTTP_3)
            .body(())?;
        let mut stream = send_request.send_request(req).await?;
        stream
            .send_data(bytes::Bytes::from_static(b"1234567890"))
            .await?;
        stream
            .send_data(bytes::Bytes::from_static(b"qwertyuiop"))
            .await?;
        stream
            .send_data(bytes::Bytes::from_static(b"asdfghjkl"))
            .await?;
        stream
            .send_data(bytes::Bytes::from_static(b"zxcvbnm"))
            .await?;
        stream.finish().await?;
        let resp = stream.recv_response().await?;
        if resp.status() != hyper::StatusCode::OK {
            return Err(Error::StatusErr);
        }
        let mut buf = bytes::BytesMut::with_capacity(1024).writer();
        while let Some(chunk) = stream.recv_data().await? {
            buf.write(chunk.chunk())?;
        }
        let data = buf.into_inner().freeze();
        if data != "1234567890qwertyuiopasdfghjklzxcvbnm" {
            return Err(Error::InvalidData);
        } else {
            Ok(())
        }
    };
    let (req_res, _drive_res) = tokio::join!(request, drive);
    req_res?;
    endpoints.wait_idle().await;
    Ok(())
}

async fn parallel() {
    let certs = coral_net::tls::TlsParam::new(
        Some("/root/certs/ecs/ca".into()),
        "/root/certs/ecs/client.crt".into(),
        "/root/certs/ecs/client.key".into(),
    );
    // let certs = coral_net::tls::TlsParam::new(
    //     Some("/root/certs/ca".into()),
    //     "/root/certs/client.crt".into(),
    //     "/root/certs/client.key".into(),
    // );
    let tls_conf = coral_net::tls::client_conf(&certs).unwrap();
    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_conf).unwrap(),
    ));
    let addr = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::new(111, 229, 180, 248)),
        9001,
        // std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        // 9001,
    );
    let mut tasks = vec![];
    for _ in 0..100 {
        let conf = client_config.clone();
        let addr = addr.clone();
        tasks.push(tokio::spawn(async move {
            if let Err(e) = handle(conf, addr).await {
                println!("{:?}", e);
            }
        }));
    }
    join_all(tasks).await;
}

#[test]
fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(parallel());
}
