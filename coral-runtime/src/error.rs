use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("get core ids is none")]
    NoneCoreIds,
    #[error("get core ids out of bounds")]
    OutBoundsCoreIds,
    #[error("can not get thread name")]
    NoneThreadName,
    #[error("can not get index from thread name")]
    NoneThreadIndex,
    #[error("failed to build async runtime")]
    BuildErr(#[from] std::io::Error),
}
