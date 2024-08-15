#![allow(unused)]
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    IoErr(#[from] std::io::Error),

    #[error("log kv error")]
    LogKvErr(#[from] log::kv::Error),

    #[error("")]
    LogSetErr(#[from] log::SetLoggerError),

    #[error("prost error")]
    ProstErr(#[from] prost::EncodeError),

    #[error("invalid log directory")]
    InvalidLogDir,
}
