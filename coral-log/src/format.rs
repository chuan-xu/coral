use std::{any::TypeId, cell::RefCell, marker::PhantomData};

use prost::Message;
use tracing::{span, Event, Subscriber};
use tracing_subscriber::{
    field::RecordFields,
    fmt::{format, FormattedFields, MakeWriter},
    layer::{self, Context, SubscriberExt},
    registry::LookupSpan,
    Registry,
};

use crate::{
    proto::{ProtoEvent, ProtoFields},
    record_proto::Record,
};

pub struct Layer<S, N = ProtoFields, E = ProtoEvent, W = fn() -> std::io::Stdout> {
    make_writer: W,
    fmt_fields: N,
    fmt_event: E,
    // fmt_span: format::FmtSpanConfig,
    // is_ansi: bool,
    log_internal_errors: bool,
    _inner: PhantomData<fn(S)>,
}

pub struct FmtContext<'a, S, N> {
    pub(crate) ctx: Context<'a, S>,
    pub(crate) fmt_fields: &'a N,
    pub(crate) event: &'a Event<'a>,
}

impl<S> Default for Layer<S> {
    fn default() -> Self {
        Self {
            fmt_fields: ProtoFields::default(),
            fmt_event: ProtoEvent::default(),
            make_writer: std::io::stdout,
            log_internal_errors: false,
            _inner: PhantomData,
        }
    }
}

impl<S, N, E, W> Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    // N: for<'writer> FormatFields<'writer> + 'static,
    N: FormatFields + 'static,
    E: FormatEvent<S, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    #[inline]
    fn make_ctx<'a>(&'a self, ctx: Context<'a, S>, event: &'a Event<'a>) -> FmtContext<'a, S, N> {
        FmtContext {
            ctx,
            fmt_fields: &self.fmt_fields,
            event,
        }
    }
}

impl<S, N, E, W> layer::Layer<S> for Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    // N: for<'writer> FormatFields<'writer> + 'static,
    // E: FormatEvent<S, N> + 'static,
    N: FormatFields + 'static,
    E: FormatEvent<S, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<FormattedFields<N>>().is_none() {
            let mut fields = FormattedFields::<N>::new(String::new());
            if self
                .fmt_fields
                // .format_fields(fields.as_writer().with_ansi(self.is_ansi), attrs)
                .format_fields(attrs)
                .is_ok()
            {
                // fields.was_ansi = self.is_ansi;
                extensions.insert(fields);
            } else {
                eprintln!(
                    "[tracing-subscriber] Unable to format the following event, ignoring: {:?}",
                    attrs
                );
            }
        }

        // 涉及 fmt_span的都先取消
        // if self.fmt_span.fmt_timing
        //     && self.fmt_span.trace_close()
        //     && extensions.get_mut::<Timings>().is_none()
        // {
        //     extensions.insert(Timings::new());
        // }

        // if self.fmt_span.trace_new() {
        //     with_event_from_span!(id, span, "message" = "new", |event| {
        //         drop(extensions);
        //         drop(span);
        //         self.on_event(&event, ctx);
        //     });
        // }
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
        let span = ctx.span(span).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(fields) = extensions.get_mut::<FormattedFields<N>>() {
            let _ = self.fmt_fields.add_fields(fields, values);
            return;
        }

        let mut fields = FormattedFields::<N>::new(String::new());
        if self
            .fmt_fields
            // .format_fields(fields.as_writer().with_ansi(self.is_ansi), values)
            .format_fields(values)
            .is_ok()
        {
            // fields.was_ansi = self.is_ansi;
            extensions.insert(fields);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: layer::Context<'_, S>) {
        thread_local! {
            // static BUF: RefCell<String> = RefCell::new(String::new());
            static BUF: RefCell<Record> = RefCell::new(Record::default());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let mut buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = Record::default();
                    &mut b
                }
            };

            let ctx = self.make_ctx(ctx, event);
            if self
                .fmt_event
                .format_event(
                    &ctx,
                    buf,
                    event,
                )
                .is_ok()
            {
                let mut writer = self.make_writer.make_writer_for(event.metadata());
                let c = buf.encode_to_vec();
                let res = std::io::Write::write_all(&mut writer, &c);
                if self.log_internal_errors {
                    if let Err(e) = res {
                        eprintln!("[tracing-subscriber] Unable to write an event to the Writer for this Subscriber! Error: {}\n", e);
                    }
                }
            } else if self.log_internal_errors {
                let err_msg = format!("Unable to format the following event. Name: {}; Fields: {:?}\n",
                    event.metadata().name(), event.fields());
                let mut writer = self.make_writer.make_writer_for(event.metadata());
                let res = std::io::Write::write_all(&mut writer, err_msg.as_bytes());
                if let Err(e) = res {
                    eprintln!("[tracing-subscriber] Unable to write an \"event formatting error\" to the Writer for this Subscriber! Error: {}\n", e);
                }
            }

            buf.clear();
        });
    }

    fn on_enter(&self, id: &span::Id, ctx: layer::Context<'_, S>) {
        // fmt_span TODO
        // if self.fmt_span.trace_enter() || self.fmt_span.trace_close() && self.fmt_span.fmt_timing {
        //     let span = ctx.span(id).expect("Span not found, this is a bug");
        //     let mut extensions = span.extensions_mut();
        //     if let Some(timings) = extensions.get_mut::<Timings>() {
        //         let now = Instant::now();
        //         timings.idle += (now - timings.last).as_nanos() as u64;
        //         timings.last = now;
        //     }

        //     if self.fmt_span.trace_enter() {
        //         with_event_from_span!(id, span, "message" = "enter", |event| {
        //             drop(extensions);
        //             drop(span);
        //             self.on_event(&event, ctx);
        //         });
        //     }
        // }
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
            _ if id == TypeId::of::<E>() => Some(&self.fmt_event as *const E as *const ()),
            _ if id == TypeId::of::<N>() => Some(&self.fmt_fields as *const N as *const ()),
            _ if id == TypeId::of::<W>() => Some(&self.make_writer as *const W as *const ()),
            _ => None,
        }
    }
}

pub trait FormatEvent<S, N>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    /// Write a log message for `Event` in `Context` to the given [`Writer`].
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        rp: &mut Record,
        event: &Event<'_>,
    ) -> Result<(), ()>;
}

pub trait FormatFields {
    /// Format the provided `fields` to the provided [`Writer`], returning a result.
    fn format_fields<R: RecordFields>(
        &self,
        // writer: Writer<'writer>,
        fields: R,
    ) -> Result<(), ()>;

    /// Record additional field(s) on an existing span.
    ///
    /// By default, this appends a space to the current set of fields if it is
    /// non-empty, and then calls `self.format_fields`. If different behavior is
    /// required, the default implementation of this method can be overridden.
    fn add_fields(
        &self,
        current: &mut FormattedFields<Self>,
        fields: &span::Record<'_>,
    ) -> Result<(), ()> {
        if !current.fields.is_empty() {
            current.fields.push(' ');
        }
        self.format_fields(fields)
    }
}
