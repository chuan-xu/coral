//! 在`tracing::subscriber::set_global_default(subscriber)`后`tokio::spawn`会重复记录span的问题
//! 多线程runtime任务出现堆积，tracing的span会重复记录

use bytes::BufMut;
use std::sync::{Arc, Mutex};
use tracing::{info, Level};

/// 创建一个span，并使用spawn提交异步任务
async fn create_span(v: i32, tx: tokio::sync::mpsc::Sender<u8>) {
    let tspan = tracing::span!(Level::INFO, "create_span", v = v, "some msg");
    let _guard = tspan.enter();
    tokio::spawn(async move {
        info!("hello");
        tx.send(0).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

#[test]
fn repeat_in_spawn() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let writer = super::LogWriter::new();
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .with_target(false)
            .with_writer(writer.clone())
            .with_max_level(Level::INFO)
            .with_ansi(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<u8>(16);
        tokio::spawn(create_span(1, tx.clone()));
        tokio::spawn(create_span(2, tx.clone()));
        rx.recv().await;
        rx.recv().await;
        let res = writer.read();
        let mut repeat = false;
        for i in res.iter() {
            // 出现两次v的赋值
            if i.matches("v=").count() > 1 {
                repeat = true;
                break;
            }
        }
        assert!(repeat);
    });
}
