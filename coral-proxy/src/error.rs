use coral_runtime::Error as RuntimeErr;
use rustls::server::VerifierBuilderError;
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("runtime error")]
    RuntimeErr(#[from] RuntimeErr),

    #[error("invalid ca directory")]
    InvalidCa,

    #[error("Io Error")]
    IoErr(#[from] std::io::Error),

    #[error("missing log directory")]
    MissingLogDir,

    #[error("invalid log directory")]
    InvalidLogDir,

    #[error("missing log rotation")]
    MissingLogRotation,

    #[error("failed to build ca certificate")]
    CaBuildErr(#[from] VerifierBuilderError),

    #[error("{0} is None")]
    NoneOption(&'static str),

    #[error("hyper inner error")]
    HyperInner(#[from] hyper::Error),

    #[error("hyper http inner error")]
    HttpInner(#[from] hyper::http::Error),

    #[error("heartbeat failed")]
    HeartBeatFailed,
}
