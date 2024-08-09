use bytes::BufMut;
use prost::Message;
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("env error")]
    VarErr(#[from] std::env::VarError),

    #[error("io error")]
    IoErr(#[from] std::io::Error),

    #[error("prost error")]
    ProstErr(#[from] prost::DecodeError),
}

fn parse(
    nums: usize,
    buf: bytes::buf::Writer<bytes::BytesMut>,
) -> Result<Vec<crate::record_proto::Record>, Error> {
    let mut res = Vec::new();
    let data = buf.into_inner().freeze();
    let mut i = 0;
    let mut num = 0;
    while i < data.len() && num < nums {
        let s: [u8; 8] = data[i..i + 8].try_into().unwrap();
        let size = u64::from_be_bytes(s) as usize;
        i += 8;
        let r = crate::record_proto::Record::decode(&data[i..i + size])?;
        res.push(r);
        i += size;
        num += 1;
    }
    Ok(res)
}

fn parse_file() -> Result<(), Error> {
    if let Ok(file_path) = env::var("LOG_FILE") {
        let nums = match env::var("LOG_NUMS") {
            Ok(v) => usize::from_str_radix(&v, 10).unwrap_or(usize::MAX),
            Err(_) => usize::MAX,
        };
        let mut fd = std::fs::File::open(file_path)?;
        let mut buf = bytes::BytesMut::with_capacity(1024).writer();
        std::io::copy(&mut fd, &mut buf)?;
        let res = parse(nums, buf)?;
        println!("{:?}", res);
    }
    Ok(())
}

#[test]
fn parse_run() {
    let res = parse_file();
    assert!(res.is_ok());
}
