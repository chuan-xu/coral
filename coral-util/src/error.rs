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

    #[error("failed to create cache client")]
    CacheCreateErr,

    #[error("failed to build tls conf")]
    TlsCfgErr(#[from] rustls::Error),

    #[error("failed to create cache subscriber")]
    CacheSubscribeErr,

    #[error("failed to publish by cacher")]
    CachePublishErr,

    #[error("failed to get by cacher")]
    CacheGetErr,

    #[error("failed to set by cacher")]
    CacheSetErr,
}
