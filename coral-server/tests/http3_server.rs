use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::sync::Arc;

use axum::routing::post;
use axum::Router;
use bytes::Buf;
use bytes::Bytes;
use coral_runtime::tokio;
use h3::error::ErrorLevel;
use h3::quic::BidiStream;
use h3::quic::RecvStream;
use h3::server::RequestStream;
use hyper::Request;
use tower::Service;

async fn hand() -> &'static str {
    "hello"
}

fn run_router() -> Router {
    let r: Router = Router::new().route("/hand", post(hand));
    r
}

// type h3_recv<T> = RequestStream<<T as BidiStream<Bytes>>::RecvStream, Bytes>;
type H3Recv<T> = RequestStream<T, Bytes>;

pin_project_lite::pin_project! {
    struct Recv<T> {
        #[pin]
        inner: H3Recv<T>,
    }
}

unsafe impl<T> Send for Recv<T> {}
unsafe impl<T> Sync for Recv<T> {}

impl<T> hyper::body::Body for Recv<T>
where T: RecvStream
{
    type Data = bytes::Bytes;

    type Error = String;

    // pin project
    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        // TODO use ready
        let this = self.project();
        let mut inner = this.inner;
        let res = futures::ready!(inner.poll_recv_data(cx)).unwrap();
        let d = res.unwrap();
        let db = d.chunk().to_vec();

        // must conver to Bytes
        // let d_pre = d.chunk();
        let r_b = bytes::Bytes::from(db);

        let res = http_body::Frame::data(r_b);
        std::task::Poll::Ready(Some(Ok(res)))
    }
}

async fn handle_request<T>(req: Request<()>, mut stream: RequestStream<T, Bytes>)
where T: BidiStream<Bytes> + 'static {
    println!("method: {:?}", req.method());
    println!("header: {:?}", req.headers());
    println!("version: {:?}", req.version());
    println!("uri: {:?}", req.uri());
    // recv data
    while let Some(data) = stream.recv_data().await.unwrap() {
        let cont = std::str::from_utf8(data.chunk()).unwrap();
        println!("{:?}", cont);
    }

    // test split
    let (mut send, recv) = stream.split();
    // while let Some(data) = recv.recv_data().await.unwrap() {
    //     let cont = std::str::from_utf8(data.chunk()).unwrap();
    //     println!("{:?}", cont);
    // }

    // merge router

    // let router: Router = Router::new().route("/hand", post(hand));
    let rre = Recv { inner: recv };
    let r_req = Request::builder().body(rre).unwrap();
    let mut r = run_router();
    let _fut = r.call(r_req);

    // recv.c

    let resp = hyper::http::Response::builder().body(()).unwrap();
    // recv data
    // stream.send_response(resp).await.unwrap();

    // test split
    send.send_response(resp).await.unwrap();
    send.send_data(bytes::Bytes::from("hello from server"))
        .await
        .unwrap();
    send.finish().await.unwrap();
}

async fn server() {
    let param = coral_net::tls::TlsParam {
        tls_ca: Some(String::from("/root/certs/ca")),
        tls_cert: String::from("/root/certs/server.crt"),
        tls_key: String::from("/root/certs/server.key"),
    };
    let tls_config = coral_net::tls::server_conf(&param).unwrap();
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn_proto::crypto::rustls::QuicServerConfig::try_from(tls_config).unwrap(),
    ));
    let addr = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 4443));
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
