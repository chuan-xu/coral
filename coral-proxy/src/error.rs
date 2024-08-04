use coral_runtime::Error as RuntimeErr;
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("runtime error")]
    RuntimeErr(#[from] RuntimeErr),
    #[error("ca_dir is none")]
    MissingCa,
    #[error("Io Error")]
    IoErr(#[from] std::io::Error),
    #[error("missing log directory")]
    MissingLogDir,
}
