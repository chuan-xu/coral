use std::sync::{Arc, Mutex};

use bytes::BufMut;

mod exception;
mod parse;
mod proto;

pub struct LogWriter {
    buf: Arc<Mutex<bytes::buf::Writer<bytes::BytesMut>>>,
}

impl Clone for LogWriter {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
        }
    }
}

impl LogWriter {
    pub fn new() -> Self {
        let container = bytes::BytesMut::with_capacity(1024).writer();
        Self {
            buf: Arc::new(Mutex::new(container)),
        }
    }

    pub fn read(&self) -> Vec<String> {
        let mut container = self.buf.lock().unwrap();
        let data = container.get_mut();
        let fmt = std::str::from_utf8(&data).unwrap();
        let mut res = Vec::new();
        for i in fmt.split("\n") {
            res.push(i.to_string());
        }
        res
    }
}

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut container = self.buf.lock().unwrap();
        container.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
