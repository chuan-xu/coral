#![allow(unused)]
use thiserror::Error;

pub(crate) type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    IoErr(#[from] std::io::Error),
}
