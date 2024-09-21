use bytes::Bytes;
use h3::quic::BidiStream;
use h3::server::RequestStream;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
    sync::Arc,
};

async fn handle_request<T>(req: Request<()>, stream: RequestStream<T, Bytes>)
where
    T: BidiStream<Bytes> + 'static,
{
    println!("method: {:?}", req.method());
    println!("header: {:?}", req.headers());
    println!("version: {:?}", req.version());
    println!("uri: {:?}", req.uri());

    let (mut send, _recv) = stream.split();
    let resp = hyper::http::Response::builder().body(()).unwrap();
    send.send_response(resp).await.unwrap();
    send.send_data(bytes::Bytes::from("hello from server"))
        .await
        .unwrap();
    send.finish().await.unwrap();
}

use coral_runtime::tokio;
use h3::error::ErrorLevel;
use hyper::Request;
async fn server() {
    let param = coral_net::tls::TlsParam {
        tls_ca: Some(String::from("/root/coral/cicd/self_sign_cert/ca")),
        tls_cert: String::from("/root/coral/cicd/self_sign_cert/server.crt"),
        tls_key: String::from("/root/coral/cicd/self_sign_cert/server.key"),
    };
    let tls_config = coral_net::tls::server_conf(&param).unwrap();
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn_proto::crypto::rustls::QuicServerConfig::try_from(tls_config).unwrap(),
    ));
    // let addr = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 9001));
    let addr = SocketAddr::from_str("[::1]:4433").unwrap();
    let endpoint = quinn::Endpoint::server(server_config, addr).unwrap();
    while let Some(new_conn) = endpoint.accept().await {
        println!("new conn");
        tokio::spawn(async {
            let conn = new_conn.await.unwrap();
            let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
                .await
                .unwrap();
            loop {
                match h3_conn.accept().await {
                    Ok(Some((req, stream))) => {
                        tokio::spawn(async {
                            handle_request(req, stream).await;
                        });
                    }

                    // indicating no more streams to be received
                    Ok(None) => {
                        break;
                    }

                    Err(err) => {
                        println!("error on accept {}", err);
                        match err.get_error_level() {
                            ErrorLevel::ConnectionError => break,
                            ErrorLevel::StreamError => continue,
                        }
                    }
                }
            }
        });
    }
}

#[test]
fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(server());
}
