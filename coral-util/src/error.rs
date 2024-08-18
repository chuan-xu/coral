use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create cache client")]
    CachelCreateErr,

    #[error("failed to create cache subscriber")]
    CachelSubscribeErr,
}
