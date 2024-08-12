//! 测试protobuf格式的日志

use std::cell::RefCell;

use bytes::BufMut;
use tracing::{
    info, info_span,
    subscriber::{set_default, DefaultGuard},
    Instrument,
};
use tracing_subscriber::{layer::SubscriberExt, Layer};

use crate::format;

#[test]
fn multi_span() {
    let writer = super::LogWriter::new();
    let subscriber = crate::proto_subscriber(writer.clone());
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let sp1 = info_span!("span1");
    let _guard1 = sp1.enter();
    let sp2 = info_span!("span2");
    let _guard2 = sp2.enter();
    info!("log info");
    drop(_guard2);
    info!("log info");
    let data = writer.read_to_bytes();
    let res = super::parse::parse_slice(usize::MAX, &data).unwrap();
    assert!(res[0].spans.len() == 2);
    assert!(res[1].spans.len() == 1);
}

async fn create_span(diff: u8) {
    let sp = info_span!("create_span", v = diff);
    let _guard = sp.enter();
    tokio::spawn(log_record());
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
}

async fn log_record() {
    info!("log info");
}

#[test]
/// 第二条日志将会重复记录span
/// 第三条日志将会重复记录两次span
fn async_multi_span() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(3)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let writer = super::LogWriter::new();
        let subscriber = crate::proto_subscriber(writer.clone());
        tracing::subscriber::set_global_default(subscriber).unwrap();
        tokio::spawn(create_span(1));
        tokio::spawn(create_span(2));
        tokio::spawn(create_span(3));
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let data = writer.read_to_bytes();
        let res = super::parse::parse_slice(usize::MAX, &data).unwrap();
        assert!(res[1].spans.len() == 2);
        assert!(res[2].spans.len() == 3);
    });
}

async fn log_record1() {
    info!("log info");
}

async fn create_span1(diff: u8) {
    let sp = info_span!("create_span", v = diff);
    let _guard = sp.enter();
    tokio::spawn(log_record1().in_current_span());
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
}

#[test]
/// 避免重复记录span
fn avoid_repeat_span() {
    let writer_hander = crate::WriterHandler::fileout(
        "/tmp",
        "coral_log.text",
        tracing_appender::rolling::Rotation::NEVER,
    );
    let writer = writer_hander.get_writer();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .on_thread_start(move || {
            thread_local! {
                static SUB_GUARD: RefCell<Option<DefaultGuard>> = RefCell::new(None);
            }
            let layer = format::Layer::new(writer.clone());
            let layered = tracing_subscriber::Registry::default().with(layer);
            let trace =
                tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL.with_subscriber(layered);
            let guard = set_default(trace);
            SUB_GUARD
                .try_with(move |v| {
                    *v.borrow_mut() = Some(guard);
                })
                .unwrap();
        })
        .build()
        .unwrap();
    rt.block_on(async {
        tokio::spawn(create_span1(1));
        tokio::spawn(create_span1(2));
        tokio::spawn(create_span1(3));
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let mut fd = std::fs::File::open("/tmp/coral_log.text").unwrap();
        let mut buf = bytes::BytesMut::with_capacity(1024).writer();
        std::io::copy(&mut fd, &mut buf).unwrap();
        let res = super::parse::parse_bytes(usize::MAX, buf).unwrap();
        for i in res.iter() {
            println!("==========");
            println!("{:?} ===> {:?}", i.spans, i.spans.len());
        }
    });
}
