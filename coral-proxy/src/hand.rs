use axum::{
    body::BodyDataStream,
    extract::Request,
    http::{uri::PathAndQuery, HeaderMap, HeaderValue},
};
use coral_log::{
    error,
    tracing::{span, Level, Span},
};
use hyper::client::conn::http2::{Connection, SendRequest};
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc,
};

use coral_runtime::tokio;

use crate::error::{CoralRes, Error};

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

struct PxyConnGuard {
    inner: Arc<AtomicU64>,
}

impl Drop for PxyConnGuard {
    fn drop(&mut self) {
        self.inner.fetch_sub(1, Ordering::AcqRel);
    }
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
    fn get_sender(&self) -> (SendRequest<BodyDataStream>, PxyConnGuard) {
        self.count.fetch_add(1, Ordering::AcqRel);
        let guard = PxyConnGuard {
            inner: self.count.clone(),
        };

        (self.sender.clone(), guard)
    }

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
        pxy_conn.heartbeat().await?;
        Ok(pxy_conn)
    }

    async fn keep_conn(conn: HandshakeConn, addr: String, state: Arc<AtomicU8>) {
        if let Err(err) = conn.await {
            error!(e = err.to_string(), addr = addr, "Proxy disconnect");
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

pub struct PxyPool {
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
        loop {
            if let Ok(conn) = PxyConn::new(&addr).await {
                let mut conns = self.inner.write().await;
                conns.push(conn);
            } else {
                error!(address = addr, "failed to reconn");
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    pub async fn build(addrs: &Vec<String>) -> CoralRes<Self> {
        let pool = PxyPool {
            inner: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        };
        for addr in addrs.iter() {
            pool.add(addr.as_str()).await?;
        }
        Ok(pool)
    }

    async fn balance(&self) -> Option<PxyConn> {
        let conns = self.inner.read().await;
        let mut conn = None;
        let mut max = u64::MAX;
        for item in conns.iter() {
            let state = item.state.load(Ordering::Acquire);
            match state {
                0 => {
                    let count = item.count.load(Ordering::Acquire);
                    if count < max {
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
        conn
    }

    async fn remove(self) {
        let mut conns = self.inner.write().await;
        conns.retain(|conn| conn.state.load(Ordering::Acquire) == PROXY_CLEANED);
    }
}

fn record_trace_id(headers: &HeaderMap<HeaderValue>) -> CoralRes<Span> {
    match headers.get("x-trace-id") {
        Some(hv) => {
            let trace_id = hv.to_str()?.to_owned();
            Ok(span!(Level::INFO, "trace_id_h", v = trace_id))
        }
        None => {
            let trace_id = uuid::Uuid::new_v4().to_string();
            Ok(span!(Level::INFO, "trace_id_s", v = trace_id))
        }
    }
}

pub async fn proxy(req: Request) -> CoralRes<hyper::Response<hyper::body::Incoming>> {
    let span = record_trace_id(req.headers())?;
    let _guard = span.enter();
    let uri = req
        .extensions()
        .get::<PathAndQuery>()
        .ok_or_else(|| {
            error!("PathAndQuery is none");
            Error::NoneOption("PathAndQuery ")
        })?
        .clone();
    let pxy_pool = req
        .extensions()
        .get::<PxyPool>()
        .ok_or_else(|| {
            error!("PxyPool is none");
            Error::NoneOption("PxyPool")
        })?
        .clone();
    let headers = req.headers().clone();
    let body = req.into_body().into_data_stream();
    let mut trans_builder = hyper::Request::builder().method("POST").uri(uri);
    let trans_headers = trans_builder.headers_mut().ok_or_else(|| {
        error!("faile to get trans header");
        Error::NoneOption("trans header")
    })?;
    *trans_headers = headers;
    let trans_req = trans_builder.body(body).map_err(|err| {
        error!(e = err.to_string(), "failed to build trans body");
        err
    })?;
    let pxy_conn = pxy_pool.balance().await.ok_or_else(|| {
        error!("pxy_conn get balance is none");
        Error::NoneOption("pxy_conn")
    })?;
    let (mut sender, _guard) = pxy_conn.get_sender();
    let rsp = sender.send_request(trans_req).await.map_err(|err| {
        error!(e = err.to_string(), "Forwarding request failed");
        err
    })?;
    Ok(rsp)
}
