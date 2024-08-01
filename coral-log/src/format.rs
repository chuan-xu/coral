// use tracing_subscriber::fmt::format::Format;

pub mod record {
    include!(concat!(".", "/record.rs"));
}

// struct Protobuf;

// struct Format(Protobuf);

// impl<'writer> tracing_subscriber::fmt::format::FormatFields<'writer> for Protobuf {
//     fn format_fields<R: tracing_subscriber::prelude::__tracing_subscriber_field_RecordFields>(
//         &self,
//         writer: tracing_subscriber::fmt::format::Writer<'writer>,
//         fields: R,
//     ) -> std::fmt::Result {
//         todo!()
//     }
// }

struct ProtoFields;

fn fmt1() {
    // let f = tracing_subscriber::fmt().pretty().with_timer();
    // let f1 = tracing_subscriber::fmt().compact().with_max_level();
    // let f = Format::default();
    // tracing_subscriber::FmtSubscriber::builder()
    //     .event_format()
    //     .fmt_fields()
    //     .with_span_events();
}

#[cfg(test)]
mod test {
    // use super::record;
    #[test]
    fn test_proto() {
        let r = record::Record {};
    }
}
