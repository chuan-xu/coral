use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
pub use tracing_appender::rolling::Rotation;
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

    pub fn fileout(directory: &str, prefix: &str, rotation: Option<Rotation>) -> Self {
        let rotation = match rotation {
            Some(r) => r,
            None => Rotation::DAILY,
        };
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

pub fn subscriber(
    writer: NonBlocking,
) -> tracing_subscriber::layer::Layered<
    tracing::level_filters::LevelFilter,
    tracing_subscriber::layer::Layered<
        format::Layer<tracing_subscriber::Registry, NonBlocking>,
        tracing_subscriber::Registry,
    >,
> {
    let layer = format::Layer::new(writer);
    let layered = tracing_subscriber::Registry::default().with(layer);
    tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL.with_subscriber(layered)
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
    fn test_proto() {
        let hand = WriterHandler::fileout("/root/tmp/log", "test.log", Some(Rotation::DAILY));
        let l = format::Layer::new(hand.writer);
        let l1 = tracing_subscriber::Registry::default().with(l);
        let f = tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL;
        let t = f.with_subscriber(l1);
        tracing::subscriber::set_global_default(t).unwrap();
        // snay();
        let v = 11;
        let span = tracing::span!(Level::INFO, "my_span", val = v, "some message");
        let _guard = span.enter();
        let span1 = tracing::span!(Level::INFO, "my_span1");
        let _guard1 = span1.enter();
        drop(_guard1);
        tracing::event!(Level::ERROR, name = "luli", "in event");
        tracing::event!(Level::ERROR, name = "luli", "in event1");
        // println!("finish");
    }

    #[test]
    fn parse_log() {
        let mut f = std::fs::File::open("/root/tmp/log/proto.log.2024-08-04").unwrap();
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

    #[test]
    fn async_log() {
        let hand = WriterHandler::fileout("/root/tmp/log", "test.log", Some(Rotation::DAILY));
        let l = format::Layer::new(hand.writer);
        let l1 = tracing_subscriber::Registry::default().with(l);
        let f = tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL;
        let t = f.with_subscriber(l1);
        tracing::subscriber::set_global_default(t).unwrap();
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
