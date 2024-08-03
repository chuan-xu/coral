use std::sync::atomic::AtomicPtr;

use chrono::format::{Fixed, Item};
use tracing::{Level, Metadata};

use crate::record_proto::{self, Fields, Meta};

pub fn create_sync_fields() -> AtomicPtr<Fields> {
    let fields = Box::leak(Box::new(Fields::default()));
    AtomicPtr::new(fields)
}

impl record_proto::Record {
    pub(crate) fn format_metadata(&mut self, meta: &Metadata) {
        self.format_timestamp();

        self.format_level(meta.level());

        if let Some(file) = meta.file() {
            self.file = file.to_owned();
        }

        if let Some(line) = meta.line() {
            self.line = line;
        }
    }

    pub(crate) fn format_timestamp(&mut self) {
        let t = chrono::Local::now();
        self.timestamp = t
            .format_with_items(core::iter::once(Item::Fixed(Fixed::RFC3339)))
            .to_string();
    }

    pub(crate) fn format_level(&mut self, level: &Level) {
        match level.as_str() {
            "TRACE" => self.level = 0,
            "DEBUG" => self.level = 1,
            "INFO" => self.level = 2,
            "WARN" => self.level = 3,
            "ERROR" => self.level = 4,
            _ => {
                //TODO
            }
        }
    }

    pub(crate) fn format_thread_name(&mut self) {
        if let Some(name) = std::thread::current().name() {
            self.thread_name = name.to_string();
        }
    }

    pub(crate) fn format_file(&mut self, file: Option<&str>) {
        if let Some(f) = file {
            self.file = f.to_owned();
        }
    }

    pub(crate) fn format_line(&mut self, line: Option<u32>) {
        if let Some(v) = line {
            self.line = v;
        }
    }

    pub(crate) fn format_event(&mut self, event: &tracing::Event<'_>) {
        let mut event_fields = Fields::default();
        event.record(&mut event_fields);
        let mut meta = record_proto::Meta::new(event.metadata().name());
        meta.fields = event_fields.take();
        self.event = Some(meta);
    }
}

impl Meta {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
        }
    }
}

impl Fields {
    fn add_field(&mut self, kind: i32, key: &str, val: String) {
        self.inner.push(record_proto::Field {
            kind,
            key: key.to_string(),
            val,
        });
    }

    pub fn take(&mut self) -> Vec<record_proto::Field> {
        std::mem::take(&mut self.inner)
    }
}

impl tracing::field::Visit for Fields {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.add_field(2, field.name(), value.to_string());
        // self.record_debug(field, &value)
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.add_field(1, field.name(), value.to_string());
        // self.record_debug(field, &value)
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.add_field(1, field.name(), value.to_string());
        // self.record_debug(field, &value)
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.add_field(0, field.name(), value.to_string());
        // self.record_debug(field, &value)
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.add_field(3, field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.add_field(3, field.name(), format!("{:?}", value));
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_proto() {
        // let r = record::Record {};
    }
}
