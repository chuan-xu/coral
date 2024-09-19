use axum::http::uri::InvalidUri;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use coral_runtime::Error as RuntimeErr;
use hyper::header::InvalidHeaderValue;
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("runtime error")]
    RuntimeErr(#[from] RuntimeErr),

    #[error("Io Error")]
    IoErr(#[from] std::io::Error),

    #[error("{0} is None")]
    NoneOption(&'static str),

    // #[error("header {0} is None")]
    // MissingHeader(&'static str),
    #[error("hyper inner error")]
    HyperInner(#[from] hyper::Error),

    #[error("hyper http inner error")]
    HttpInner(#[from] hyper::http::Error),

    #[error("invalid uri")]
    InvalidUri(#[from] InvalidUri),

    #[error("failed to conver str to header")]
    HeaderFromStrErr(#[from] InvalidHeaderValue),

    #[error("coral log error")]
    CoralLogErr(#[from] coral_log::error::Error),

    #[error("coral net module error")]
    CoralNetErr(#[from] coral_net::error::Error),

    #[error("serde json error")]
    JsonErr(#[from] serde_json::error::Error),

    #[error("miss conn pool")]
    MissPool,

    #[error("conn pool is empty")]
    EmptyPool,

    #[error("axum inner error")]
    AxumErr(#[from] axum::Error),

    #[error("utf8 error")]
    Utf8Err(#[from] std::str::Utf8Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self)).into_response()
    }
}
