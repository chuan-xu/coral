use axum::response::IntoResponse;
use coral_runtime::Error as RuntimeErr;
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
}

#[derive(Debug)]
pub enum CoralErr {}

impl std::fmt::Display for CoralErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CoralErr")
    }
}

impl std::error::Error for CoralErr {}

impl IntoResponse for CoralErr {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}
