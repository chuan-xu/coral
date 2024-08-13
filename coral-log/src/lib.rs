pub use tracing;
use tracing::subscriber::set_global_default;
pub use tracing_appender::non_blocking::NonBlocking;
// pub use tracing::{self, debug, error, info, instrument, subscriber::DefaultGuard, warn,
// Level};
use tracing_appender::non_blocking::WorkerGuard;
pub use tracing_appender::rolling::Rotation;
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

mod error;
mod format;
mod proto;
#[cfg(debug_assertions)]
mod record_proto;
#[cfg(not(debug_assertions))]
pub mod record_proto {
    include!(concat!(".", "/record_proto.rs"));
}

#[cfg(test)]
mod tests;

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

pub fn proto_subscriber<W>(
    w: W,
) -> tracing_subscriber::layer::Layered<
    tracing::level_filters::LevelFilter,
    tracing_subscriber::layer::Layered<
        format::Layer<tracing_subscriber::Registry, W>,
        tracing_subscriber::Registry,
    >,
>
where W: for<'writer> MakeWriter<'writer> + 'static {
    let layerd = tracing_subscriber::Registry::default().with(format::Layer::new(w));
    tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL.with_subscriber(layerd)
}

pub fn str_subscriber<W>(
    w: W,
    with_ansi: bool,
) -> tracing_subscriber::FmtSubscriber<
    tracing_subscriber::fmt::format::DefaultFields,
    tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Compact, ChronoLocal>,
    tracing::level_filters::LevelFilter,
    W,
>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    tracing_subscriber::FmtSubscriber::builder()
        .compact()
        .with_timer(ChronoLocal::rfc_3339())
        .with_ansi(with_ansi)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .with_thread_names(true)
        .with_writer(w)
        .finish()
}

pub fn subscriber(is_debug: bool, writer: NonBlocking) {
    match is_debug {
        false => {
            let layer = format::Layer::new(writer);
            let layered = tracing_subscriber::Registry::default().with(layer);
            let trace =
                tracing_subscriber::FmtSubscriber::DEFAULT_MAX_LEVEL.with_subscriber(layered);
            set_global_default(trace).unwrap();
        }
        true => {
            let time_fmt = tracing_subscriber::fmt::time::ChronoLocal::rfc_3339();
            let trace = tracing_subscriber::FmtSubscriber::builder()
                .compact()
                .with_timer(time_fmt)
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .with_level(true)
                .with_thread_names(true)
                .with_writer(writer)
                .finish();
            set_global_default(trace).unwrap();
        }
    }
}
