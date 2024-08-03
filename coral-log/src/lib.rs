use tracing::Level;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
pub use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt, layer::SubscriberExt, Layer, Registry};

mod error;
mod format;
mod proto;
#[cfg(debug_assertions)]
mod record_proto;
#[cfg(not(debug_assertions))]
pub mod record_proto {
    include!(concat!(".", "/record_proto.rs"));
}

type Format = fmt::format::Format<fmt::format::Compact, fmt::time::ChronoLocal>;

pub struct LogWriterHandler {
    format: Format,
    writer: NonBlocking,
    _guard: WorkerGuard,
}

impl LogWriterHandler {
    pub fn stdout() -> Self {
        let (stdout, _guard) = tracing_appender::non_blocking(std::io::stdout());
        Self {
            format: Self::format(true),
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
            format: Self::format(false),
            writer: fileout,
            _guard,
        }
    }

    pub fn get_writer(&self) -> NonBlocking {
        self.writer.clone()
    }

    fn format(
        is_debug: bool,
    ) -> tracing_subscriber::fmt::format::Format<
        tracing_subscriber::fmt::format::Compact,
        tracing_subscriber::fmt::time::ChronoLocal,
    > {
        tracing_subscriber::fmt::format()
            .compact()
            .with_ansi(is_debug)
            .with_target(true)
            .with_level(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
    }
}

// fn test() {
//     use tracing_subscriber::Layer;
//     let l1 = format::Layer::default();
//     let l2 = tracing_subscriber::Registry::default().with(l1);
//     let f = tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL;
//     let l3 = f.with_subscriber(l2);
// }

pub fn subscriber(is_debug: bool) {
    let level = match is_debug {
        true => Level::TRACE,
        false => Level::INFO,
    };
    let subscriber = tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL
        .with_subscriber(Registry::default().with(format::Layer::default()));
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
