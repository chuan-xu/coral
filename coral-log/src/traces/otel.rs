use fastrace::collector::Config;
use fastrace_opentelemetry;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;

pub fn otel_trace<T: IntoIterator<Item = KeyValue>>(endpoint: String, kvs: T) {
    let reporter = fastrace_opentelemetry::OpenTelemetryReporter::new(
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint)
            .with_protocol(opentelemetry_otlp::Protocol::Grpc)
            .with_timeout(std::time::Duration::from_secs(
                opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT,
            ))
            .build_span_exporter()
            .expect("initialize oltp exporter"),
        opentelemetry::trace::SpanKind::Server,
        std::borrow::Cow::Owned(opentelemetry_sdk::Resource::new(kvs)),
        opentelemetry::InstrumentationLibrary::builder(env!("CARGO_PKG_NAME"))
            .with_version(env!("CARGO_PKG_VERSION"))
            .build(),
    );
    fastrace::set_reporter(reporter, Config::default());
}
