use std::convert::Infallible;

use bytes::Bytes;
use coral_runtime::tokio;
use http_body_util::StreamBody;
use hyper::body::Frame;
use hyper_util::rt::TokioIo;
use tokio_stream::wrappers::ReceiverStream;

type Data = Result<Frame<Bytes>, Infallible>;

async fn handle() {
    let addr = "127.0.0.1:8080";
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(async move {
        if let Err(err) = conn.await {
            println!("{:?}", err);
        }
    });
    let (tx, rx) = tokio::sync::mpsc::channel::<Data>(10);
    tokio::spawn(async move {
        tx.send(Ok(Frame::data(Bytes::from_static(b"hello"))))
            .await
            .unwrap();
        // std::thread::park();
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        tx.send(Ok(Frame::data(Bytes::from_static(b"world"))))
            .await
            .unwrap();
    });
    let stream = ReceiverStream::new(rx);
    let stream_body = StreamBody::new(stream);

    let req = hyper::Request::builder()
        .uri("http://127.0.0.1:8080")
        .method("POST")
        .version(hyper::Version::HTTP_11)
        .body(stream_body)
        .unwrap();
    let _rsp = sender.send_request(req).await.unwrap();
    std::thread::park();
}

#[test]
fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(handle());
}
