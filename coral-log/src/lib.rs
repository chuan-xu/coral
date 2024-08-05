use tracing::subscriber::set_default;
pub use tracing::{self, debug, error, info, instrument, subscriber::DefaultGuard, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
pub use tracing_appender::{non_blocking::NonBlocking, rolling::Rotation};
use tracing_subscriber::{layer::SubscriberExt, Layer};

mod error;
mod format;
mod proto;
#[cfg(debug_assertions)]
mod record_proto;
#[cfg(not(debug_assertions))]
pub mod record_proto {
    include!(concat!(".", "/record_proto.rs"));
}

pub struct WriterHandler {
    writer: NonBlocking,
    _guard: WorkerGuard,
}

impl WriterHandler {
    pub fn stdout() -> Self {
        let (stdout, _guard) = tracing_appender::non_blocking(std::io::stdout());
        Self {
            writer: stdout,
            _guard,
        }
    }

    pub fn fileout(directory: &str, prefix: &str, rotation: Rotation) -> Self {
        let appender =
            tracing_appender::rolling::RollingFileAppender::new(rotation, directory, prefix);
        let (fileout, _guard) = tracing_appender::non_blocking(appender);
        Self {
            writer: fileout,
            _guard,
        }
    }

    pub fn get_writer(&self) -> NonBlocking {
        self.writer.clone()
    }
}

pub fn subscriber(is_debug: bool, writer: NonBlocking) -> DefaultGuard {
    match is_debug {
        true => {
            let layer = format::Layer::new(writer);
            let layered = tracing_subscriber::Registry::default().with(layer);
            let trace =
                tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL.with_subscriber(layered);
            set_default(trace)
        }
        false => {
            let time_fmt = tracing_subscriber::fmt::time::ChronoLocal::rfc_3339();
            let trace = tracing_subscriber::FmtSubscriber::builder()
                .pretty()
                .with_timer(time_fmt)
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .with_level(true)
                .with_thread_names(true)
                .with_writer(writer)
                .finish();
            set_default(trace)
        }
    }
}

#[cfg(test)]
mod tests {

    use bytes::BufMut;
    use prost::Message;
    use tracing::{info, instrument, Level};
    use tracing_appender::rolling::Rotation;
    use tracing_subscriber::{layer::SubscriberExt, Layer};

    use crate::{
        format,
        record_proto::{self},
        WriterHandler,
    };

    #[tracing::instrument]
    fn snay() {
        info!("hello world");
    }

    #[test]
    #[ignore = "manual test"]
    fn test_proto() {
        let hand = WriterHandler::fileout("/root/tmp/log", "test.log", Rotation::DAILY);
        let l = format::Layer::new(hand.writer);
        let l1 = tracing_subscriber::Registry::default().with(l);
        let f = tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL;
        let t = f.with_subscriber(l1);
        let _subscriber_guard = tracing::subscriber::set_default(t);
        let v = 11;
        let span = tracing::span!(Level::INFO, "my_span", val = v, "some message");
        let _guard = span.enter();
        let span1 = tracing::span!(Level::INFO, "my_span1");
        let _guard1 = span1.enter();
        drop(_guard1);
        tracing::event!(Level::ERROR, name = "luli", "in event");
        tracing::event!(Level::ERROR, name = "luli", "in event1");
    }

    #[test]
    #[ignore = "manual test"]
    fn parse_log() {
        let mut f = std::fs::File::open("/root/tmp/log/test.log.2024-08-05").unwrap();
        let mut buf = bytes::BytesMut::with_capacity(1024).writer();
        std::io::copy(&mut f, &mut buf).unwrap();
        let b = buf.into_inner().freeze();
        let mut i = 0;
        while i < b.len() {
            let s: [u8; 8] = b[i..i + 8].try_into().unwrap();
            let size = u64::from_be_bytes(s) as usize;
            i += 8;
            let r = record_proto::Record::decode(&b[i..i + size]).unwrap();
            println!("{:?}", r);
            i += size;
        }
    }

    #[instrument]
    async fn f1(_val: u32) {
        info!("before sleep");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        info!(val = _val, "after sleep");
    }

    #[instrument]
    fn param(age: Option<i32>) {
        let name = Some(String::from("hello world"));
        info!(name = name, "hello from param");
    }

    #[test]
    #[ignore = "manual test"]
    fn test_enum() {
        let hand = WriterHandler::fileout("/root/tmp/log", "test.log", Rotation::DAILY);
        let layer = format::Layer::new(hand.writer);
        let layered = tracing_subscriber::Registry::default().with(layer);
        let subscriber =
            tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL.with_subscriber(layered);
        let _subscriber_guard = tracing::subscriber::set_default(subscriber);
        param(Some(13));
    }

    #[test]
    #[ignore = "manual test"]
    fn async_log() {
        let hand = WriterHandler::fileout("/root/tmp/log", "test.log", Rotation::DAILY);
        let l = format::Layer::new(hand.writer);
        let l1 = tracing_subscriber::Registry::default().with(l);
        let f = tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL;
        let t = f.with_subscriber(l1);
        let _subscriber_guard = tracing::subscriber::set_default(t);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(3)
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            tokio::spawn(f1(11));
            tokio::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                println!("after sleep 4");
            });
            tokio::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(100)).await;
                println!("after sleep 4");
            });
            tokio::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(100)).await;
                println!("after sleep 4");
            });
        });
    }
}
