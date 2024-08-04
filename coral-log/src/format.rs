use std::{
    any::TypeId,
    cell::RefCell,
    marker::PhantomData,
    sync::atomic::{AtomicPtr, Ordering},
};

use bytes::BufMut;
use prost::Message;
use tracing::{span, Event, Subscriber};
use tracing_subscriber::{fmt::MakeWriter, layer, registry::LookupSpan};

use crate::{
    proto::create_sync_fields,
    record_proto::{self, Fields, Record},
};

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
    // N: StructFields + Default + Sync + Send + 'static,
    // E: FormatEvent<S, N> + 'static,
    // E: StructEvent<S> + Default + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<AtomicPtr<Fields>>().is_none() {
            let fields = create_sync_fields();
            if let Err(_) = fields.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| unsafe {
                if let Some(t) = v.as_mut() {
                    attrs.record(t);
                }
                Some(v)
            }) {
                eprintln!(
                    "[tracing-subscriber] Unable to format the following event, ignoring: {:?}",
                    attrs
                );
            }
            extensions.insert(fields);
        }
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
        let span = ctx.span(span).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(fields) = extensions.get_mut::<AtomicPtr<Fields>>() {
            if let Err(_) = fields.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| unsafe {
                if let Some(t) = v.as_mut() {
                    values.record(t);
                }
                Some(v)
            }) {
                eprintln!(
                    "[tracing-subscriber] Unable to format the following event, ignoring: {:?}",
                    values
                );
            }
            return;
        }

        let fields = create_sync_fields();
        if let Err(_) = fields.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| unsafe {
            if let Some(t) = v.as_mut() {
                values.record(t);
            }
            Some(v)
        }) {
            eprintln!(
                "[tracing-subscriber] Unable to format the following event, ignoring: {:?}",
                values
            );
        }
        extensions.insert(fields);
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
                let ext = span.extensions();
                if let Some(fields) = ext.get::<AtomicPtr<Fields>>() {
                    if let Err(_) = fields.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| unsafe {
                        if let Some(t) = v.as_mut() {
                            proto_span.fields = t.take();
                        }
                        Some(v)
                    }) {
                        eprintln!(
                            "[tracing-subscriber] Unable to format the following event, ignoring",
                        );
                    }
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
