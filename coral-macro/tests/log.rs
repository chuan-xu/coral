use fastrace::collector::Config;
use fastrace::collector::ConsoleReporter;
use fastrace::local::LocalSpan;
use fastrace::prelude::SpanContext;
use fastrace::Span;

struct Tlog {
    data: *mut String,
}

unsafe impl Send for Tlog {}
unsafe impl Sync for Tlog {}

impl log::Log for Tlog {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        unsafe {
            self.data.write(format!("{:?}", record));
        }
    }

    fn flush(&self) {}
}

#[test]
#[ignore = "single"]
fn log_level_error() {
    let mut data = String::new();
    log::set_boxed_logger(Box::new(Tlog { data: &mut data })).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    coral_macro::trace_error!("hello");
    assert!(data.contains("level: Error"));
}

#[test]
#[ignore = "single"]
fn log_level_warn() {
    let mut data = String::new();
    log::set_boxed_logger(Box::new(Tlog { data: &mut data })).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    coral_macro::trace_warn!("hello");
    assert!(data.contains("level: Warn"));
}
#[test]
#[ignore = "single"]
fn log_level_info() {
    let mut data = String::new();
    log::set_boxed_logger(Box::new(Tlog { data: &mut data })).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    coral_macro::trace_info!("hello");
    assert!(data.contains("level: Info"));
}
#[test]
#[ignore = "single"]
fn log_level_debug() {
    let mut data = String::new();
    log::set_boxed_logger(Box::new(Tlog { data: &mut data })).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    coral_macro::trace_debug!("hello");
    assert!(data.contains("level: Debug"));
}
#[test]
#[ignore = "single"]
fn log_level_trace() {
    let mut data = String::new();
    log::set_boxed_logger(Box::new(Tlog { data: &mut data })).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    coral_macro::trace_trace!("hello");
    assert!(data.contains("level: Trace"));
}

#[test]
#[ignore = "single"]
fn macro_trace() {
    let mut data = String::new();
    log::set_boxed_logger(Box::new(Tlog { data: &mut data })).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    fastrace::set_reporter(ConsoleReporter, Config::default());
    let parent = SpanContext::random();
    let span = Span::root("root", parent);
    let _guard = span.set_local_parent();
    let _child = LocalSpan::enter_with_local_parent("child");
    let curr = SpanContext::current_local_parent().unwrap();
    let age = 11;
    coral_macro::trace_debug!(name = "hello", age = age; "hello {}", "todo");
    let trace_id = curr.trace_id.0.to_string();
    assert!(data.contains(&trace_id));
}
