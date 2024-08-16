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

use bytes::BufMut;
use log::info;

use crate::logs::logger::Logger;
use crate::logs::logs_proto::{self, Record};
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

#[test]
fn test_disk() {
    let fd = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("/root/tmp/benchlog.log")
        .unwrap();
    let logger = Logger::<Record>::new(log::Level::Info, None, fd).unwrap();
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    let mut ths = Vec::new();
    for i in 0..4 {
        let th_name = String::from("th-") + i.to_string().as_str();

        ths.push(
            std::thread::Builder::new()
                .name(th_name)
                .spawn(|| {
                    for _ in 0..250000 {
                        info!(e = "some err info"; "XXX-xxx-aaa");
                    }
                })
                .unwrap(),
        );
    }
    while let Some(th) = ths.pop() {
        th.join().unwrap();
    }
    println!("finish");
}

#[test]
#[ignore = "manual"]
fn test_nums() {
    let mut fd = std::fs::File::open("/root/tmp/benchlog.log").unwrap();
    let mut buf = bytes::BytesMut::with_capacity(1024).writer();
    std::io::copy(&mut fd, &mut buf).unwrap();
    let records = parse::parse_bytes(usize::MAX, buf).unwrap();
    assert_eq!(records.len(), 1000000);
}
