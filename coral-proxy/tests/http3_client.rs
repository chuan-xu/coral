use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use coral_net::tls::client_conf;
use coral_runtime::tokio;
use coral_runtime::tokio::io::AsyncWriteExt;

async fn client() -> Result<(), Box<dyn std::error::Error>> {
    let param = coral_net::tls::TlsParam {
        tls_ca: Some(String::from("/root/certs/ca")),
        tls_cert: String::from("/root/certs/client.crt"),
        tls_key: String::from("/root/certs/client.key"),
    };

    let host = "server.test.com";

    // let addr = tokio::net::lookup_host((host, 9001))
    //     .await
    //     .unwrap()
    //     .next()
    //     .ok_or("dns found no addresses")
    //     .unwrap();
    let addr = SocketAddr::new(
        // std::net::IpAddr::V4(Ipv4Addr::new(111, 229, 180, 248)),
        // 9001,
        std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        9001,
    );

    println!("addr: {:?}", addr);

    let tls_config = client_conf(&param).unwrap();

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config).unwrap(),
    ));

    let mut client_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse().unwrap()).unwrap();
    client_endpoint.set_default_client_config(client_config);

    let conn = client_endpoint.connect(addr, host).unwrap().await.unwrap();

    let quinn_conn = h3_quinn::Connection::new(conn);

    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await.unwrap();

    let drive = async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx)).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    let request = async move {
        println!("sending request ...");

        let uri = "https://server.test.com:9001/benchmark".parse::<hyper::http::Uri>()?;
        // let uri = "/test_http3".parse::<hyper::http::Uri>()?;

        let req = hyper::http::Request::builder()
            .method("POST")
            .uri(uri)
            .header(hyper::header::CONTENT_LENGTH, "36")
            .body(())?;
        println!("###################################");

        // sending request results in a bidirectional stream,
        // which is also used for receiving response
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

        // finish on the sending side
        stream.finish().await?;

        println!("receiving response ...");

        let resp = stream.recv_response().await?;

        println!("response: {:?} {}", resp.version(), resp.status());
        println!("headers: {:#?}", resp.headers());

        while let Some(mut chunk) = stream.recv_data().await? {
            let mut out = tokio::io::stdout();
            out.write_all_buf(&mut chunk).await?;
            out.flush().await?;
        }

        Ok::<_, Box<dyn std::error::Error>>(())
    };

    let (req_res, drive_res) = tokio::join!(request, drive);
    req_res?;
    drive_res?;

    // wait for the connection to be closed before exiting
    client_endpoint.wait_idle().await;
    Ok(())
}

#[test]
fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(client()).unwrap();
}
