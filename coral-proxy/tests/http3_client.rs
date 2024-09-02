use std::sync::Arc;

use coral_runtime::tokio;
use coral_runtime::tokio::io::AsyncWriteExt;
use coral_util::tls::client_conf;

async fn client() -> Result<(), Box<dyn std::error::Error>> {
    let param = coral_util::cli::CommParam {
        cache_addr: None,
        ca_dir: Some(String::from("/root/certs/ca")),
        certificate: String::from("/root/certs/client.crt"),
        private_key: String::from("/root/certs/client.key"),
    };

    let host = "server.test.com";

    let addr = tokio::net::lookup_host((host, 4443))
        .await
        .unwrap()
        .next()
        .ok_or("dns found no addresses")
        .unwrap();

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

        let uri = "https://server.test.com:4443/test_http3".parse::<hyper::http::Uri>()?;
        // let uri = "/test_http3".parse::<hyper::http::Uri>()?;

        let req = hyper::http::Request::builder().uri(uri).body(())?;
        println!("###################################");

        // sending request results in a bidirectional stream,
        // which is also used for receiving response
        let mut stream = send_request.send_request(req).await?;

        // send empty body
        // stream
        //     .send_data(bytes::Bytes::from("hello from client"))
        //     .await
        //     .unwrap();

        // finish on the sending side
        stream.finish().await?;

        println!("receiving response ...");

        let resp = stream.recv_response().await?;

        println!("response: {:?} {}", resp.version(), resp.status());
        println!("headers: {:#?}", resp.headers());

        // `recv_data()` must be called after `recv_response()` for
        // receiving potential response body
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
