//! 测试protobuf格式的日志

use std::sync::{Arc, Mutex};

use bytes::{BufMut, BytesMut};

#[test]
fn multi_span() {
    let writer = super::LogWriter::new();
    let subscriber = crate::proto_subscriber(writer);
}
