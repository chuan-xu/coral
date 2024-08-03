#![allow(unused)]
use thiserror::Error;

pub type CoralRes<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {}
