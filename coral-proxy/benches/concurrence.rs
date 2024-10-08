use std::io::Write;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;
use coral_net::tls::TlsConf;
use coral_runtime::spawn;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use futures::future::join_all;
use h3::client::SendRequest;
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

async fn request(mut sender: SendRequest<h3_quinn::OpenStreams, Bytes>) -> Result<(), Error> {
    let req = hyper::http::Request::builder()
        .method("POST")
        // .uri("https://server.test.com:9001/benchmark")
        .uri("https://tx.coral.com:9001/benchmark")
        .header(hyper::header::CONTENT_LENGTH, "36")
        .version(hyper::Version::HTTP_3)
        .body(())?;
    let mut stream = sender.send_request(req).await?;
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
}

async fn handle(conf: quinn::ClientConfig, addr: SocketAddr) -> Result<(), Error> {
    let mut endpoints = quinn::Endpoint::client("[::]:0".parse().unwrap()).unwrap();
    endpoints.set_default_client_config(conf);
    let conn = endpoints
        .connect(addr, "tx.coral.com")
        .unwrap()
        .await
        .unwrap();
    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, sender) = h3::client::new(quinn_conn).await.unwrap();
    let drive = async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx))
            .await
            .unwrap();
    };
    spawn(drive);
    for _ in 0..500 {
        let sender = sender.clone();
        spawn(async move {
            if let Err(err) = request(sender).await {
                println!("{:?}", err);
            }
        });
    }
    drop(sender);
    endpoints.wait_idle().await;
    Ok(())
}

fn bench(c: &mut Criterion) {
    c.bench_function("Concurrent test", |b| {
        let rt = coral_runtime::tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        b.to_async(rt).iter(|| async {
            let toml_str = r#"
                ca = "/root/certs/ecs/ca"
                cert = "/root/certs/ecs/client.crt"
                key = "/root/certs/ecs/client.key"
            "#;
            let tls_conf: TlsConf = toml::from_str(toml_str).unwrap();
            let tls_client_conf = tls_conf.client_conf().unwrap();
            let client_config = quinn::ClientConfig::new(Arc::new(
                quinn::crypto::rustls::QuicClientConfig::try_from(tls_client_conf).unwrap(),
            ));
            let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9001);
            let mut tasks = vec![];
            for _ in 0..20 {
                let conf = client_config.clone();
                let addr = addr.clone();
                tasks.push(spawn(async move {
                    if let Err(e) = handle(conf, addr).await {
                        println!("{:?}", e);
                    }
                }));
            }
            join_all(tasks).await;
        });
    });
}

fn get_config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group!(name = benches; config = get_config(); targets = bench);
criterion_main!(benches);
