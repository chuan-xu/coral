use std::{any::TypeId, cell::RefCell, marker::PhantomData};

use bytes::BufMut;
use prost::Message;
use tracing::{span, Event, Subscriber};
use tracing_subscriber::{fmt::MakeWriter, layer, registry::LookupSpan};

use crate::record_proto::{self, Fields, Record};

pub struct Layer<S, W = fn() -> std::io::Stdout> {
    writer: W,
    log_internal_errors: bool,
    _inner: PhantomData<fn(S)>,
}

impl<S> Default for Layer<S> {
    fn default() -> Self {
        Self {
            writer: std::io::stdout,
            log_internal_errors: false,
            _inner: PhantomData,
        }
    }
}

impl<S, W> Layer<S, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            log_internal_errors: true,
            _inner: PhantomData,
        }
    }
}

impl<S, W> layer::Layer<S> for Layer<S, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if extensions.get_mut::<Fields>().is_none() {
            let mut fields = Fields::default();
            attrs.record(&mut fields);
            extensions.insert(fields);
        }
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
        let span = ctx.span(span).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(fields) = extensions.get_mut::<Fields>() {
            values.record(fields);
        } else {
            let mut fields = Fields::default();
            values.record(&mut fields);
            extensions.insert(fields);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: layer::Context<'_, S>) {
        thread_local! {
            static BUF: RefCell<Record> = RefCell::new(Record::default());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = Record::default();
                    &mut b
                }
            };

            let meta = event.metadata();
            buf.format_metadata(meta);
            buf.format_timestamp();
            buf.format_level(meta.level());
            buf.format_thread_name();
            buf.format_file(meta.file());
            buf.format_line(meta.line());
            buf.format_event(event);
            let spans = event.parent().and_then(|id| ctx.span(id)).or_else(|| ctx.lookup_current());
            let scope = spans.into_iter().flat_map(|span| span.scope().from_root());
            for span in scope{
                let mut proto_span = record_proto::Meta::new(span.name());
                let mut ext = span.extensions_mut();
                if let Some(fields) = ext.get_mut::<Fields>() {
                    // take之后嵌套的span无法记录原来的值
                    // proto_span.fields = fields.take();
                    proto_span.fields = fields.inner();
                }
                buf.spans.push(proto_span);
            }
            let enc_data = buf.encode_to_vec();
            let mut bytes_buf = bytes::BytesMut::with_capacity(1024);
            bytes_buf.put_u64(enc_data.len() as u64);
            bytes_buf.put_slice(&enc_data);
            let enc_bytes = bytes_buf.freeze();
            let mut writer = self.writer.make_writer();
            let res = std::io::Write::write_all(&mut writer, &enc_bytes);
            if self.log_internal_errors {
                if let Err(e) = res {
                    eprintln!("[tracing-subscriber] Unable to write an event to the Writer for this Subscriber! Error: {}\n", e);
                }
            }
            *buf = Record::default();
        });
    }

    fn on_exit(&self, _id: &span::Id, _ctx: layer::Context<'_, S>) {
        // fmt_span TODO
    }

    fn on_close(&self, _id: span::Id, _ctx: layer::Context<'_, S>) {
        // fmt_span TODO
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        // This `downcast_raw` impl allows downcasting a `fmt` layer to any of
        // its components (event formatter, field formatter, and `MakeWriter`)
        // as well as to the layer's type itself. The potential use-cases for
        // this *may* be somewhat niche, though...
        match () {
            _ if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
            _ if id == TypeId::of::<W>() => Some(&self.writer as *const W as *const ()),
            _ => None,
        }
    }
}
