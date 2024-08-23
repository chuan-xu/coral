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
}
