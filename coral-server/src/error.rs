use axum::response::IntoResponse;
use coral_runtime::Error as RuntimeErr;
use hyper::{header::InvalidHeaderValue, StatusCode};
use thiserror::Error;

pub type CoralRes<T> = Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("runtime error")]
    RuntimeErr(#[from] RuntimeErr),

    #[error("Io Error")]
    IoErr(#[from] std::io::Error),

    #[error("coral log error")]
    CoralLogErr(#[from] coral_log::error::Error),

    #[error("invald net address")]
    AddrErr(#[from] std::net::AddrParseError),

    #[error("coral net module error")]
    CoralNetErr(#[from] coral_net::error::Error),

    #[error("h3 error")]
    H3Err(#[from] h3::Error),

    #[error("{0} is None")]
    NoneOption(&'static str),

    #[error("failed to conver str to header")]
    HeaderFromStrErr(#[from] InvalidHeaderValue),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self)).into_response()
    }
}
