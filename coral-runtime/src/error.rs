use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("get core ids is none")]
    NoneCoreIds,
    #[error("get core ids out of bounds")]
    OutBoundsCoreIds,
}
