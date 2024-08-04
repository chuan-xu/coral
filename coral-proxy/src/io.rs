use axum::{body::BodyDataStream, routing::post, Router};
use coral_runtime::tokio;
use futures::StreamExt;
use hyper::{body::Incoming, client::conn::http2::SendRequest};
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::{
    cell::{LazyCell, RefCell},
    sync::Arc,
};
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::{cli, error::CoralRes, hand, tls};

pub struct PxyChan {
    sender: SendRequest<BodyDataStream>,
    count: Arc<()>,
}

impl PxyChan {
    async fn new(dst: &String) -> PxyChan {
        let stream = tokio::net::TcpStream::connect(dst).await.unwrap();
        let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
            .handshake(TokioIo::new(stream))
            .await
            .unwrap();
        tokio::spawn(async move {
            conn.await.unwrap();
        });
        let body = axum::body::Body::empty().into_data_stream();
        let req = hyper::Request::builder()
            .method("POST")
            .uri("/heartbeat")
            .body(body)
            .unwrap();
        let res = sender.send_request(req).await.unwrap();
        if res.status() != hyper::StatusCode::OK {
            std::panic!("invalid server addr");
        }
        Self {
            sender,
            count: Arc::default(),
        }
    }

    fn ref_count(&self) -> usize {
        Arc::strong_count(&self.count)
    }

    pub fn get_sender(&mut self) -> &mut SendRequest<BodyDataStream> {
        &mut self.sender
    }
}

impl Clone for PxyChan {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            count: self.count.clone(),
        }
    }
}

// pub async fn proxy_client(addr: &str) -> hyper::client::conn::http2::SendRequest<BodyDataStream> {
//     println!("{:?}", addr);
//     let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
//     let (send, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
//         .handshake(TokioIo::new(stream))
//         .await
//         .unwrap();
//     tokio::spawn(async move {
//         conn.await.unwrap();
//     });
//     send
// }

async fn hand_stream(
    tls_accept: TlsAcceptor,
    cnx: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    tower_service: Router,
    pxy_chan: PxyChan,
) {
    match tls_accept.accept(cnx).await {
        Ok(stream) => {
            // TODO 先用TokioIo 后面自定义实现
            let stream = TokioIo::new(stream);
            let hyper_service =
                hyper::service::service_fn(move |mut req: hyper::Request<Incoming>| {
                    let uri = req.uri();
                    let path = uri.path_and_query().unwrap().to_owned();
                    let nuri = match uri.scheme_str() {
                        Some(scheme_str) => {
                            let mut scheme = scheme_str.to_string();
                            scheme += "://";
                            scheme += uri.authority().unwrap().as_str();
                            hyper::Uri::try_from(scheme).unwrap()
                        }
                        None => hyper::Uri::from_static("/"),
                    };
                    *(req.uri_mut()) = nuri;
                    req.extensions_mut().insert(path);
                    req.extensions_mut().insert(pxy_chan.clone());
                    tower_service.clone().call(req)
                });
            let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(stream, hyper_service)
                .await;
            if let Err(err) = ret {
                println!("error serving connection from {}: {}", addr, err);
            }
        }
        Err(_) => {
            // TODO
        }
    }
}

async fn tcp_accept(
    app: &Router,
    tls_acceptor: &TlsAcceptor,
    tcp_listener: &tokio::net::TcpListener,
    pxy_chan: &Vec<PxyChan>,
) {
    let tower_service = app.clone();
    let tls_accept = tls_acceptor.clone();
    let ch = pxy_chan
        .iter()
        .min_by_key(|v| v.ref_count())
        .unwrap()
        .clone();
    match tcp_listener.accept().await {
        Ok((cnx, addr)) => {
            tokio::spawn(hand_stream(tls_accept, cnx, addr, tower_service, ch));
        }
        Err(_) => todo!(),
    }
}

async fn server(args: cli::Cli) {
    let conf = tls::server_conf(&args).unwrap();
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(conf);
    let bind = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), args.port);
    let tcp_listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    let app = Router::new().route("/", post(hand::proxy));
    let pxy_chan: Vec<_> = futures::stream::iter(&args.addresses)
        .then(|item| PxyChan::new(item))
        .collect()
        .await;

    futures::pin_mut!(tcp_listener);
    loop {
        tcp_accept(&app, &tls_acceptor, &tcp_listener, &pxy_chan).await;
    }
}

fn before(is_debug: bool) {
    std::thread_local! {
        static _SUBSCRIBER_GUARD: RefCell<Option<coral_log::DefaultGuard>> = RefCell::new(None);
    }
    if is_debug {
    } else {
    }
}

pub fn run() -> CoralRes<()> {
    let args = cli::parse()?;
    let rt = coral_runtime::runtime(args.cpui, args.nums, "coral-proxy", || {})?;
    rt.block_on(server(args));
    Ok(())
}
