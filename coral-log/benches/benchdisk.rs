use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;

#[cfg(feature = "tktrace")]
mod tktrace {

    use tracing::error;
    use tracing::info;
    use tracing::warn;

    #[tracing::instrument]
    fn callsite_1(_v1: &str, _v2: u32) {
        error!("test error msg!");
    }
    #[tracing::instrument]
    fn callsite_2() {
        warn!("test warn msg!");
    }
    #[tracing::instrument]
    fn callsite_3() {
        info!("test info msg!");
        callsite_2();
    }
    #[tracing::instrument]
    fn callsite_4() {
        callsite_1("day", 7);
        callsite_2();
        info!("finish");
    }

    fn remove_test_file(dir: &str, prefix: &str) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.file_name().to_str().unwrap().starts_with(prefix) {
                        std::fs::remove_file(entry.path()).unwrap();
                    }
                }
            }
        }
    }

    fn bench(c: &mut Criterion) {
        c.bench_function("gen_json", |b| {
            b.iter(|| {
                remove_test_file("/root/tmp/log", "json.log");
                let appender = tracing_appender::rolling::RollingFileAppender::new(
                    tracing_appender::rolling::Rotation::DAILY,
                    "/root/tmp/log",
                    "json.log",
                );
                let (writer, _guard) = tracing_appender::non_blocking(appender);
                let t = tracing_subscriber::FmtSubscriber::builder()
                    .json()
                    .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
                    .with_file(true)
                    .with_level(true)
                    .with_line_number(true)
                    .with_thread_names(true)
                    .with_max_level(tracing::Level::INFO)
                    .with_writer(writer)
                    .finish();
                let _guard = tracing::subscriber::set_default(t);
                for _ in 0..1000000 {
                    callsite_4();
                }
            });
        });
        c.bench_function("gen_compact", |b| {
            b.iter(|| {
                remove_test_file("/root/tmp/log", "compact.log");
                let appender = tracing_appender::rolling::RollingFileAppender::new(
                    tracing_appender::rolling::Rotation::DAILY,
                    "/root/tmp/log",
                    "compact.log",
                );
                let (writer, _guard) = tracing_appender::non_blocking(appender);
                let t = tracing_subscriber::FmtSubscriber::builder()
                    .compact()
                    .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
                    .with_file(true)
                    .with_level(true)
                    .with_line_number(true)
                    .with_thread_names(true)
                    .with_max_level(tracing::Level::INFO)
                    .with_writer(writer)
                    .finish();
                let _guard = tracing::subscriber::set_default(t);
                for _ in 0..1000000 {
                    callsite_4();
                }
            });
        });
        c.bench_function("gen_proto", |b| {
            remove_test_file("/root/tmp/log", "proto.log");
            b.iter(|| {
                let appender = tracing_appender::rolling::RollingFileAppender::new(
                    tracing_appender::rolling::Rotation::DAILY,
                    "/root/tmp/log",
                    "proto.log",
                );
                let (writer, _guard) = tracing_appender::non_blocking(appender);
                let _guard = coral_log::subscriber(true, writer);
                for _ in 0..1000000 {
                    callsite_4();
                }
            });
        });
    }
}

fn get_config() -> Criterion {
    Criterion::default().sample_size(10)
}

fn bench(c: &mut Criterion) {}

#[cfg(feature = "tktrace")]
criterion_group!(name = benches; config = get_config(); targets = tktrace::bench);

criterion_group!(name = benches; config = get_config(); targets = bench);
criterion_main!(benches);
