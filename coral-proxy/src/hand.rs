use axum::{body::BodyDataStream, extract::Request, http::uri::PathAndQuery};
use coral_log::error;
use hyper::client::conn::http2::{Connection, SendRequest};
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc,
};

use coral_runtime::tokio;

use crate::error::{CoralRes, Error};

pub struct PxyChan {
    sender: SendRequest<BodyDataStream>,
    count: Arc<()>,
}

/// 代理连接正常
static PROXY_NORMAL: u8 = 0;

/// 远端服务拒绝后续连接
static PROXY_REJECT: u8 = 1;

/// 远端服务已经关闭
static PROXY_CLOSED: u8 = 2;

/// 远端服务即将关闭
static PROXY_CLEANING: u8 = 3;

/// 已经无连接
static PROXY_CLEANED: u8 = 4;

struct PxyConn {
    /// 代理数据发送的句柄
    sender: SendRequest<BodyDataStream>,

    /// 当前代理连接的状态
    state: Arc<AtomicU8>,

    /// 连接的数量
    count: Arc<AtomicU64>,

    /// 服务地址
    addr: String,
}

impl Clone for PxyConn {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            state: self.state.clone(),
            count: self.count.clone(),
            addr: self.addr.clone(),
        }
    }
}

type HandshakeSend = SendRequest<BodyDataStream>;

type HandshakeConn = Connection<TokioIo<tokio::net::TcpStream>, BodyDataStream, TokioExecutor>;

type HandshakeSocket = (HandshakeSend, HandshakeConn);

async fn handshake(addr: &str) -> CoralRes<HandshakeSocket> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    let socket: HandshakeSocket = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(TokioIo::new(stream))
        .await?;
    Ok(socket)
}
impl PxyConn {
    async fn new(addr: &str) -> CoralRes<PxyConn> {
        let (sender, conn) = handshake(addr).await?;
        let state = Arc::new(AtomicU8::new(0));
        tokio::spawn(Self::keep_conn(conn, addr.to_owned(), state.clone()));
        let pxy_conn = Self {
            sender,
            state,
            count: Arc::new(AtomicU64::new(0)),
            addr: addr.to_owned(),
        };
        Ok(pxy_conn)
    }

    async fn keep_conn(conn: HandshakeConn, addr: String, state: Arc<AtomicU8>) {
        if let Err(e) = conn.await {
            error!(error = e.to_string(), addr = addr, "Proxy disconnect");
            let mut current = PROXY_NORMAL;
            loop {
                if let Err(c) = state.compare_exchange(
                    current,
                    PROXY_CLOSED,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    if c == PROXY_NORMAL || c == PROXY_REJECT {
                        current = c;
                        continue;
                    }
                }
                break;
            }
        }
    }

    async fn heartbeat(&self) -> CoralRes<()> {
        let body = axum::body::Body::empty().into_data_stream();
        let req = hyper::Request::builder()
            .method("POST")
            .uri("/heartbeat")
            .body(body)?;
        let res = self.sender.clone().send_request(req).await?;
        if res.status() != hyper::StatusCode::OK {
            return Err(Error::HeartBeatFailed);
        }
        Ok(())
    }

    async fn clean_check(self) {
        loop {
            if self.count.load(Ordering::Acquire) == 0 {
                if let Err(e) = self.state.compare_exchange(
                    PROXY_CLEANING,
                    PROXY_CLEANED,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    error!(
                        state = e,
                        "failed to compare exchange PROXY_CLEANING to PROXY_CLEANED"
                    );
                }
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }
}

struct PxyPool {
    inner: Arc<tokio::sync::RwLock<Vec<PxyConn>>>,
}

impl Clone for PxyPool {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl PxyPool {
    async fn add(&self, addr: &str) -> CoralRes<()> {
        let mut conns = self.inner.write().await;
        conns.push(PxyConn::new(addr).await?);
        Ok(())
    }

    async fn reconn(self, addr: String) {
        if let Ok(conn) = PxyConn::new(&addr).await {
            let mut conns = self.inner.write().await;
            conns.push(conn);
        } else {
            error!(address = addr, "failed to reconn");
        }
    }

    async fn build(addrs: &Vec<String>) -> CoralRes<Self> {
        let pool = PxyPool {
            inner: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        };
        for addr in addrs.iter() {
            pool.add(addr.as_str()).await?;
        }
        Ok(pool)
    }

    async fn balance(&self) -> CoralRes<Option<PxyConn>> {
        let conns = self.inner.read().await;
        let mut conn = None;
        let mut max = 0;
        for item in conns.iter() {
            let state = item.state.load(Ordering::Acquire);
            match state {
                0 => {
                    let count = item.count.load(Ordering::Acquire);
                    if count > max {
                        max = count;
                        conn = Some(item.clone());
                    }
                }
                2 => {
                    if let Err(e) = item.state.compare_exchange(
                        PROXY_CLOSED,
                        PROXY_CLEANING,
                        Ordering::SeqCst,
                        Ordering::Acquire,
                    ) {
                        error!(
                            state = e,
                            "failed to compare exchange PROXY_CLOSED to PROXY_CLEANING"
                        );
                        continue;
                    }
                    tokio::spawn(self.clone().reconn(item.addr.clone()));
                    tokio::spawn(item.clone().clean_check());
                }
                4 => {
                    tokio::spawn(self.clone().remove());
                }
                _ => continue,
            }
        }
        Ok(conn)
    }

    async fn remove(self) {}
}

impl PxyChan {
    pub async fn new(dst: &String) -> CoralRes<PxyChan> {
        let stream = tokio::net::TcpStream::connect(dst).await?;
        let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
            .handshake(TokioIo::new(stream))
            .await?;

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                error!(error = e.to_string(), "proxy chan conn failed");
            }
        })
        .await
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        let body = axum::body::Body::empty().into_data_stream();
        let req = hyper::Request::builder()
            .method("POST")
            .uri("/heartbeat")
            .body(body)?;
        let res = sender.send_request(req).await?;
        if res.status() != hyper::StatusCode::OK {
            std::panic!("invalid server addr");
        }
        Ok(Self {
            sender,
            count: Arc::default(),
        })
    }

    pub fn ref_count(&self) -> usize {
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

pub async fn proxy(req: Request) -> hyper::Response<hyper::body::Incoming> {
    let uri = req.extensions().get::<PathAndQuery>().unwrap().clone();
    let mut pxy_ch = req.extensions().get::<PxyChan>().unwrap().clone();
    let headers = req.headers().clone();
    let body = req.into_body().into_data_stream();
    let mut pxy_builder = hyper::Request::builder().method("POST").uri(uri);
    let pxy_headers = pxy_builder.headers_mut().unwrap();
    *pxy_headers = headers;
    let pxy_req = pxy_builder.body(body).unwrap();
    let rsp = pxy_ch.get_sender().send_request(pxy_req).await.unwrap();
    rsp
}
