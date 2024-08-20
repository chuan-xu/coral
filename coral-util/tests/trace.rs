use fastrace::collector::Config;
use fastrace::collector::ConsoleReporter;
use fastrace::collector::SpanContext;
use fastrace::future::FutureExt;
use fastrace::Span;

#[test]
fn record_trace_id() {
    fastrace::set_reporter(ConsoleReporter, Config::default());
    let parent = SpanContext::random();
    let span = Span::root("root", parent);
    let _guard = span.set_local_parent();
    let current = SpanContext::current_local_parent();
    assert!(current.is_some());
}

#[test]
fn spawn_record_trace_id() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let f = async {
        fastrace::set_reporter(ConsoleReporter, Config::default());
        let parent = SpanContext::random();
        let span = Span::root("root", parent);
        let sf = async {
            let current = SpanContext::current_local_parent();
            assert!(current.is_some());
        }
        .in_span(span);
        tokio::spawn(sf);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    rt.block_on(f);
}
