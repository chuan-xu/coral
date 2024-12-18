use axum::response::IntoResponse;
use hyper::header::InvalidHeaderValue;
use hyper::StatusCode;
use quinn::crypto::rustls::NoInitialCipherSuite;
use rustls::pki_types::InvalidDnsNameError;
use rustls::server::VerifierBuilderError;
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid ca directory")]
    InvalidCa,

    #[error("Io Error")]
    IoErr(#[from] std::io::Error),

    #[error("failed to build ca certificate")]
    CaBuildErr(#[from] VerifierBuilderError),

    #[error("{0} is None")]
    NoneOption(&'static str),

    #[error("failed to connect discover service")]
    DiscoverConnErr,

    #[error("failed to build tls conf")]
    TlsCfgErr(#[from] rustls::Error),

    #[error("failed to create subscriber from discover service")]
    DiscoverSubscribeErr,

    #[error("failed to publish to discover service")]
    DiscoverPublishErr,

    #[error("failed to get from discover service")]
    DiscoverGetErr,

    #[error("failed to set to discover service")]
    DiscoverSetErr,

    #[error("invalid dns name")]
    InvalidDnsNameError(#[from] InvalidDnsNameError),

    #[error("hyper inner error")]
    HyperInner(#[from] hyper::Error),

    #[error("quic client config from tls_cfg")]
    QuicCfgErr(#[from] NoInitialCipherSuite),

    #[error("parse addr from str")]
    AddrParseError(#[from] std::net::AddrParseError),

    #[error("quinn proto connect error")]
    ConnectError(#[from] quinn_proto::ConnectError),

    #[error("quinn proto connect error")]
    ConnectError1(#[from] quinn_proto::ConnectionError),

    #[error("h3 error")]
    H3Err(#[from] h3::error::Error),

    #[error("heartbeat failed")]
    HeartBeatFailed,

    #[error("hyper http inner error")]
    HttpInner(#[from] hyper::http::Error),

    #[error("http invalid uri")]
    HttpUri(#[from] hyper::http::uri::InvalidUri),

    #[error("http uri with invalid authority")]
    UriAuthErr,

    #[error("tokio sync oneshot recv err")]
    OneshotRecv(#[from] coral_runtime::tokio::sync::oneshot::error::RecvError),

    #[error("empty addr in lookup host")]
    EmptyAddr,

    #[error("missing router")]
    MissRouter,

    #[error("infallible")]
    Infallible(#[from] std::convert::Infallible),

    #[error("header {0} is None")]
    MissingHeader(&'static str),

    #[error("failed to conver str to header")]
    HeaderFromStrErr(#[from] InvalidHeaderValue),

    // ---------- log ----------
    #[error("log::ParseLevelError")]
    LogParseLevelError(#[from] log::ParseLevelError),

    // ---------- db ----------
    #[error("sqlx::error")]
    SqlxErr(#[from] sqlx::Error),

    // ---------- redis ----------
    #[error("redis::RedisError")]
    RedisErr(#[from] redis::RedisError),

    #[error("invalid redis conn type")]
    InvalidConnType,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self)).into_response()
    }
}
