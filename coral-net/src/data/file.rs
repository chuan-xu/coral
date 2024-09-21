#![allow(unused)]
use axum::response::IntoResponse;
// use axum::body::BodyDataStream
use coral_runtime::tokio::fs;
pub struct File {
    zip: super::zip::Algorithm,
}

pub struct FileRsp {}

async fn f() {
    let t = fs::OpenOptions::new().read(true).open("").await.unwrap();
}
