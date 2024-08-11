//! 测试protobuf格式的日志

use tracing::{info, info_span};

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
    let data = writer.read_to_bytes();
    let res = super::parse::parse_slice(usize::MAX, &data).unwrap();
    println!("{:?}", res);
}
