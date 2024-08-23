#![allow(unused)]
use axum::http::header::ToStrError;
use axum::http::uri::InvalidUri;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use coral_runtime::Error as RuntimeErr;
use hyper::header::InvalidHeaderValue;
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("runtime error")]
    RuntimeErr(#[from] RuntimeErr),

    #[error("Io Error")]
    IoErr(#[from] std::io::Error),

    #[error("{0} is None")]
    NoneOption(&'static str),

    #[error("header {0} is None")]
    MissingHeader(&'static str),

    #[error("hyper inner error")]
    HyperInner(#[from] hyper::Error),

    #[error("hyper http inner error")]
    HttpInner(#[from] hyper::http::Error),

    #[error("heartbeat failed")]
    HeartBeatFailed,

    #[error("failed to convert header to str")]
    HeaderToStrErr(#[from] ToStrError),

    #[error("invalid uri")]
    InvalidUri(#[from] InvalidUri),

    #[error("failed to conver str to header")]
    HeaderFromStrErr(#[from] InvalidHeaderValue),

    #[error("coral log error")]
    CoralLogErr(#[from] coral_log::error::Error),

    #[error("failed to service discovery")]
    DiscoverErr,

    #[error("coral util module error")]
    CoralUtilErr(#[from] coral_util::error::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
