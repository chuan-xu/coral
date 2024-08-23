use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::sync::Arc;

use bytes::Buf;
use bytes::Bytes;
use coral_runtime::tokio;
use coral_util::tls::server_conf;
use h3::quic::BidiStream;
use h3::server::RequestStream;
use hyper::Request;

async fn handle_request<T>(req: Request<()>, mut stream: RequestStream<T, Bytes>)
where T: BidiStream<Bytes> {
    println!("method: {:?}", req.method());
    println!("header: {:?}", req.headers());
    println!("version: {:?}", req.version());
    println!("uri: {:?}", req.uri());
    while let Some(data) = stream.recv_data().await.unwrap() {
        let cont = std::str::from_utf8(data.chunk()).unwrap();
        println!("{:?}", cont);
    }
    let resp = hyper::http::Response::builder().body(()).unwrap();
    stream.send_response(resp).await.unwrap();
    stream
        .send_data(bytes::Bytes::from("hello from server"))
        .await
        .unwrap();
    stream.finish().await.unwrap();
}

async fn server() {
    let param = coral_util::cli::CommParam {
        cache_addr: None,
        ca_dir: Some(String::from("/root/certs/ca")),
        certificate: String::from("/root/certs/server.crt"),
        private_key: String::from("/root/certs/server.key"),
    };
    let tls_config = server_conf(&param).unwrap();
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn_proto::crypto::rustls::QuicServerConfig::try_from(tls_config).unwrap(),
    ));
    let addr = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 4433));
    let endpoint = quinn::Endpoint::server(server_config, addr).unwrap();

    while let Some(new_conn) = endpoint.accept().await {
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
                        // match err.get_error_level() {
                        //     ErrorLevel::ConnectionError => break,
                        //     ErrorLevel::StreamError => continue,
                        // }
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
