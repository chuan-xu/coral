#![allow(unused)]
use fastrace::collector::Config;
use fastrace::collector::ConsoleReporter;
use fastrace::prelude::SpanContext;
use fastrace::Span;
use log;
use log::info;

#[test]
fn macro_info() {
    println!("asdasdsad");
    log::set_boxed_logger(Box::new(Tlog {}));
    log::set_max_level(log::LevelFilter::Trace);
    let sv = String::from("h");
    let svs = sv.as_str();
    // info!(target: "target", name="luli", age = 11; "hello world {}", "todo");
    // info!(target: "target", "hello world {}", "todo");
    let v = 1;
    coral_macro::trace_info!(target: "", a = b.as_str(), c = d; "asdas");
    // coral_macro::trace_info!(v = "v", a = "a");
}

struct Tlog;

impl log::Log for Tlog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        todo!()
    }

    fn log(&self, record: &log::Record) {
        println!("{:?}", record);
    }

    fn flush(&self) {
        todo!()
    }
}

#[test]
fn macro_trace() {
    log::set_boxed_logger(Box::new(Tlog {}));
    log::set_max_level(log::LevelFilter::Trace);
    fastrace::set_reporter(ConsoleReporter, Config::default());
    let parent = SpanContext::random();
    let span = Span::root("root", parent);
    let _guard = span.set_local_parent();
    let current = SpanContext::current_local_parent().unwrap();
    // current.trace_id
    log::info!(trace_id = current.trace_id.0; "hello");
}
