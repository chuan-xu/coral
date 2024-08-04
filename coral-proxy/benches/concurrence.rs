use axum::body::Body;
use coral_runtime::tokio;
use criterion::{criterion_group, criterion_main, Criterion};
use futures::future::join_all;
use http_body_util::BodyExt;
use hyper::StatusCode;

use hyper_util::rt::TokioExecutor;
use rustls::ClientConfig;

fn get_client_cnf() -> ClientConfig {
    let root_file = std::fs::File::open("/root/certs/server.crt").unwrap();
    let mut buf = std::io::BufReader::new(root_file);
    let cert_iter = rustls_pemfile::certs(&mut buf).map(|v| v.unwrap());
    let mut root_cert = rustls::RootCertStore::empty();
    root_cert.add_parsable_certificates(cert_iter);
    let client_cert_file = std::fs::File::open("/root/certs/client.crt").unwrap();
    let mut buf2 = std::io::BufReader::new(client_cert_file);
    let client_key_file = std::fs::File::open("/root/certs/client.key").unwrap();
    let mut buf3 = std::io::BufReader::new(client_key_file);
    let cert_chain: Vec<_> = rustls_pemfile::certs(&mut buf2)
        .map(|v| v.unwrap())
        .collect();
    let prv_key = rustls_pemfile::private_key(&mut buf3).unwrap().unwrap();
    ClientConfig::builder()
        .with_root_certificates(root_cert)
        .with_client_auth_cert(cert_chain, prv_key)
        .unwrap()
}

async fn https(conf: ClientConfig) {
    let now = std::time::Instant::now();
    // let connector = hyper_rustls::HttpsConnectorBuilder::new()
    //     .with_tls_config(conf)
    //     .https_only()
    //     .build();
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(conf)
        .https_only()
        .enable_http2()
        .build();
    let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(connector);
    let req = hyper::Request::builder()
        .method("POST")
        .uri("https://server.test.com:9000/benchmark")
        .body(Body::empty())
        .unwrap();
    let mut res = client.request(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mut full = Vec::new();
    while let Some(next) = res.frame().await {
        let frame = next.unwrap();
        if let Some(data) = frame.data_ref() {
            full.extend_from_slice(data);
        }
    }
    if let Ok(txt) = String::from_utf8(full) {
        assert_eq!(txt, "benchmark");
    } else {
        println!("failed to convert u8 to String");
    }
    let elapsed = now.elapsed();
    if elapsed > std::time::Duration::from_secs(3) {
        println!("timeout!!!");
    }
}

fn bench(c: &mut Criterion) {
    c.bench_function("Concurrent testing", |b| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        b.to_async(rt).iter(|| async {
            let conf = get_client_cnf();
            let mut tasks = Vec::new();
            for _ in 0..1000 {
                let task = tokio::spawn(https(conf.clone()));
                tasks.push(task);
            }
            join_all(tasks).await;
        })
        // b.iter(move || {
        //     rt.block_on(async {
        //         let conf = get_client_cnf();
        //         let mut tasks = Vec::new();
        //         for _ in 0..1000 {
        //             let task = tokio::spawn(https(conf.clone()));
        //             tasks.push(task);
        //         }
        //         join_all(tasks).await;
        //     })
        // })
    });
}

fn get_config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group!(name = benches; config = get_config(); targets = bench);
criterion_main!(benches);
