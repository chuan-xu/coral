mod parse;
struct CaptureWriter {
    inner: Vec<u8>,
}

impl std::io::Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.extend_from_slice(buf);
        println!("{:?}", self.inner);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

use log::info;

use crate::logs::logger::Logger;
use crate::logs::logs_proto;
#[test]
fn check_coral_log() {
    let w = CaptureWriter { inner: Vec::new() };
    let h = Logger::<logs_proto::Record>::new(log::Level::Info, Some(1024), w).unwrap();
    log::set_boxed_logger(Box::new(h)).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    info!("nihao");
    let join = std::thread::Builder::new()
        .name(String::from("luli"))
        .spawn(|| {
            let a = 11;
            let v = String::from("nihao");
            let t = v.as_str();
            info!(key1 = t, key2 = 11; "hello {}", a);
        })
        .unwrap();
    join.join().unwrap();
}
