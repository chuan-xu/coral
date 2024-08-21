use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::body::BodyDataStream;
use axum::extract::Request;
use axum::http::uri::PathAndQuery;
use axum::Router;
use coral_macro::trace_error;
use coral_runtime::tokio;
use coral_runtime::tokio::sync::RwLock;
use coral_util::tow::add_header_span_id;
use hyper::body::Incoming;
use hyper::client::conn::http2::Connection;
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use log::error;
use tower::Service;

use crate::error::CoralRes;
use crate::error::Error;
use crate::util::get_modify_path_url;
use crate::util::HTTP_RESET_URI;

/// 代理连接正常
static PROXY_NORMAL: u8 = 0;

/// 远端服务拒绝后续连接
static PROXY_REJECT: u8 = 1;

/// 远端服务已经关闭
static PROXY_CLOSED: u8 = 2;

/// 等待所有连接句柄销毁
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
        tokio::spawn(Self::keep_conn(conn, addr.to_owned()));
        let pxy_conn = Self {
            sender,
            state,
            count: Arc::new(AtomicU64::new(0)),
            addr: addr.to_owned(),
        };
        pxy_conn.heartbeat().await?;
        Ok(pxy_conn)
    }

    async fn keep_conn(conn: HandshakeConn, addr: String) {
        if let Err(err) = conn.await {
            error!(e = err.to_string(), addr = addr.as_str(); "Proxy disconnect");
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

    /// 对已关闭的连接，检查是否还有没有释放的句柄，完全释放后将状态改为关闭状态
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
                        state = e;
                        "failed to compare exchange PROXY_CLEANING to PROXY_CLEANED"
                    );
                }
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }
}

pub struct ConnPool {
    inner: Arc<tokio::sync::RwLock<Vec<PxyConn>>>,
}

impl Clone for ConnPool {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl ConnPool {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn check_repeated(self, addr: &str) -> bool {
        let conns = self.inner.read().await;
        for conn in conns.iter() {
            if conn.addr == addr && conn.state.load(Ordering::Acquire) == PROXY_NORMAL {
                return false;
            }
        }
        return true;
    }

    async fn add(self, addr: &str) {
        // 防止创建重复的连接
        if self.clone().check_repeated(addr).await {
            match PxyConn::new(&addr).await {
                Ok(conn) => {
                    let mut conns = self.inner.write().await;
                    conns.push(conn)
                }
                Err(err) => {
                    error!(e = err.to_string(); "failed to new proxy conn");
                }
            }
        }
    }

    async fn remove(self) {
        let mut conns = self.inner.write().await;
        conns.retain(|conn| conn.state.load(Ordering::Acquire) != PROXY_CLEANED);
    }

    fn check_state(&self, conn: &PxyConn) -> u64 {
        let state = conn.state.load(Ordering::Acquire);
        let closed = conn.sender.is_closed();
        let mut res = u64::MAX;
        match state {
            0 | 1 => {
                if closed {
                    let mut cur = PROXY_NORMAL;
                    loop {
                        if let Err(c) = conn.state.compare_exchange(
                            cur,
                            PROXY_CLOSED,
                            Ordering::SeqCst,
                            Ordering::Acquire,
                        ) {
                            if c == PROXY_NORMAL || c == PROXY_REJECT {
                                cur = c;
                                continue;
                            }
                        }
                        break;
                    }
                } else if state == PROXY_NORMAL {
                    res = conn.count.load(Ordering::Acquire);
                }
            }
            2 => {
                if let Err(e) = conn.state.compare_exchange(
                    PROXY_CLOSED,
                    PROXY_CLEANING,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    error!(
                        state = e;
                        "failed to compare exchange PROXY_CLOSED to PROXY_CLEANING"
                    );
                } else {
                    tokio::spawn(conn.clone().clean_check());
                }
            }
            4 => {
                tokio::spawn(self.clone().remove());
            }
            _ => {}
        }
        res
    }

    async fn balance(self) -> Option<PxyConn> {
        let conns = self.inner.read().await;
        let mut conn = None;
        let mut max = u64::MAX;
        for item in conns.iter() {
            let res = self.check_state(item);
            if res < max {
                max = res;
                conn = Some(item.clone())
            }
        }
        conn
    }
}

pub async fn set_discover(cache_addr: Option<&String>, conn_pool: ConnPool) -> CoralRes<()> {
    if let Some(cache_addr) = cache_addr {
        let mut client = coral_util::db::cache::MiniRedis::new(cache_addr).await?;
        if let Some(vals) = client.get(coral_util::consts::REDIS_KEY_DISCOVER).await? {
            let endpoints = vals
                .split(|k| *k == 44)
                .filter_map(|k| std::str::from_utf8(k).ok())
                .collect::<Vec<&str>>();
            for end in endpoints.iter() {
                conn_pool.clone().add(&end).await;
            }
        }
        let state = Arc::new(AtomicU8::new(0));
        tokio::spawn(coral_util::db::cache::discover(
            cache_addr.to_owned(),
            vec![String::from(coral_util::consts::REDIS_KEY_NOTIFY)],
            |address, pool| async move {
                for addr in address.iter() {
                    pool.clone().add(addr).await;
                }
            },
            conn_pool,
            state.clone(),
        ));
        // 等待连接成功
        loop {
            match state.load(Ordering::Acquire) {
                0 => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
                1 => return Err(crate::error::Error::DiscoverErr),
                _ => break,
            }
        }
    }
    Ok(())
}

pub fn http_reset(
    mut req: Request<Incoming>,
    pool: ConnPool,
    mut router: Router,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    let ori_uri = req.uri();
    let (path, mod_uri) = get_modify_path_url(ori_uri, HTTP_RESET_URI).unwrap();
    let path = path.to_owned();
    *(req.uri_mut()) = mod_uri;
    req.extensions_mut().insert(path);
    req.extensions_mut().insert(pool);
    router.call(req)
}

pub async fn proxy(req: Request) -> CoralRes<hyper::Response<hyper::body::Incoming>> {
    let uri = req
        .extensions()
        .get::<PathAndQuery>()
        .ok_or_else(|| {
            error!("PathAndQuery is none");
            Error::NoneOption("PathAndQuery ")
        })?
        .clone();
    let pool = req
        .extensions()
        .get::<ConnPool>()
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
    add_header_span_id(trans_headers);

    let trans_req = trans_builder.body(body).map_err(|err| {
        error!(e = err.to_string(); "failed to build trans body");
        err
    })?;
    let pxy_conn = pool.balance().await.ok_or_else(|| {
        error!("pxy_conn get balance is none");
        Error::NoneOption("pxy_conn")
    })?;
    let (mut sender, _guard) = pxy_conn.get_sender();
    let rsp = sender.send_request(trans_req).await.map_err(|err| {
        trace_error!(e = err.to_string(); "Forwarding request failed");
        err
    })?;
    Ok(rsp)
}
