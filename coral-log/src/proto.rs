use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;

use crate::format::{FormatEvent, FormatFields};
use crate::record_proto;

#[derive(Default)]
pub struct ProtoEvent;

impl<S, N> FormatEvent<S, N> for ProtoEvent
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    // N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &crate::format::FmtContext<'_, S, N>,
        // writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> Result<(), ()> {
        todo!()
    }
}

#[derive(Default)]
pub struct ProtoFields {
    _private: (),
}

impl FormatFields for ProtoFields {
    fn format_fields<R: tracing_subscriber::prelude::__tracing_subscriber_field_RecordFields>(
        &self,
        // writer: Writer<'writer>,
        fields: R,
    ) -> Result<(), ()> {
        let mut v = ProtoVistor::new();
        fields.record(&mut v);
        Ok(())
    }
}

struct ProtoVistor<'a> {
    fields: std::collections::HashMap<&'a str, record_proto::FieldVal>,
}

impl<'a> ProtoVistor<'a> {
    fn new() -> Self {
        Self {
            fields: std::collections::HashMap::new(),
        }
    }
    fn add_field(&mut self, kind: i32, key: &'a str, val: String) {
        self.fields
            .insert(key, record_proto::FieldVal { kind, val });
    }
}

impl<'a> tracing::field::Visit for ProtoVistor<'a> {
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
        // self.record_debug(field, &value)
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.add_field(3, field.name(), format!("{:?}", value));
    }
}

fn fmt1() {
    // let f = tracing_subscriber::fmt().json().finish();
    // let f1 = tracing_subscriber::fmt().compact().with_max_level();
}

#[cfg(test)]
mod test {
    #[test]
    fn test_proto() {
        // let r = record::Record {};
    }
}
