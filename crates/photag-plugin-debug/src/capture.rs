//! One process-wide tracing/log adapter; plugin lifetime controls its weak sink.
use crate::store::{DebugService, bounded};
use std::{
    fmt::Write,
    sync::{Arc, OnceLock, RwLock, Weak},
    time::Instant,
};
use tracing::{
    Subscriber,
    span::{Attributes, Id, Record},
};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};
static SINK: OnceLock<Arc<RwLock<Weak<DebugService>>>> = OnceLock::new();

/// Install before creating Cordis/runtime so startup diagnostics are captured.
/// Repeated attachment replaces the weak sink; the adapter never owns a plugin.
pub fn attach(service: &Arc<DebugService>) -> Result<(), String> {
    let sink = SINK.get_or_init(|| Arc::new(RwLock::new(Weak::new())));
    *sink.write().map_err(|e| e.to_string())? = Arc::downgrade(service);
    static INSTALL: OnceLock<Result<(), String>> = OnceLock::new();
    INSTALL
        .get_or_init(|| {
            let previous = std::panic::take_hook();
            let panic_sink = sink.clone();
            std::panic::set_hook(Box::new(move |info| {
                if let Some(s) = panic_sink.read().ok().and_then(|s| s.upgrade()) {
                    s.record(
                        "ERROR",
                        "photag_panic",
                        "panic",
                        &info.to_string(),
                        None,
                        None,
                        None,
                    );
                }
                previous(info);
            }));
            tracing_log::LogTracer::init().map_err(|e| e.to_string())?;
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry().with(Capture { sink: sink.clone() }),
            )
            .map_err(|e| e.to_string())
        })
        .clone()
}
pub fn detach(service: &Arc<DebugService>) {
    if let Some(sink) = SINK.get() {
        let mut sink = sink.write().unwrap_or_else(|e| e.into_inner());
        if sink.ptr_eq(&Arc::downgrade(service)) {
            *sink = Weak::new();
        }
    }
}
struct Capture {
    sink: Arc<RwLock<Weak<DebugService>>>,
}
struct SpanData {
    started: Instant,
    fields: Fields,
    parent: Option<u64>,
    failed: bool,
}
#[derive(Default)]
struct Fields(String, Option<f64>);
impl tracing::field::Visit for Fields {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if field.name() == "duration_ms" {
            self.1 = Some(value);
        }
        self.record_debug(field, &value);
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // Bound accumulated fields even for pathological third-party events.
        if self.0.len() < crate::store::MAX_TEXT {
            let _ = write!(self.0, "{}={value:?} ", field.name());
            self.0 = bounded(&self.0);
        }
    }
}
impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Capture {
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _: Context<'_, S>) -> bool {
        // MCP protocol responses contain the queried log buffer itself. Exclude
        // transport payload tracing to prevent recursive self-recording.
        !metadata.target().starts_with("rmcp")
            || matches!(
                *metadata.level(),
                tracing::Level::ERROR | tracing::Level::WARN | tracing::Level::INFO
            )
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        let parent = span.parent().map(|s| s.id().into_u64());
        if let Some(s) = self.sink.read().ok().and_then(|s| s.upgrade()) {
            s.record(
                "INFO",
                span.metadata().target(),
                span.name(),
                &format!("started {}", fields.0),
                None,
                Some(id.into_u64()),
                parent,
            );
        }
        span.extensions_mut().insert(SpanData {
            started: Instant::now(),
            fields,
            parent,
            failed: false,
        });
    }
    fn on_record(&self, id: &Id, record: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            if let Some(data) = span.extensions_mut().get_mut::<SpanData>() {
                record.record(&mut data.fields);
            }
        }
    }
    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let Some(s) = self.sink.read().ok().and_then(|s| s.upgrade()) else {
            return;
        };
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        let mut fields = Fields::default();
        event.record(&mut fields);
        let span = ctx.event_span(event);
        if metadata.level() == &tracing::Level::ERROR {
            if let Some(span) = &span {
                if let Some(data) = span.extensions_mut().get_mut::<SpanData>() {
                    data.failed = true;
                }
            }
        }
        s.record(
            metadata.level().as_str(),
            metadata.target(),
            span.as_ref().map(|s| s.name()).unwrap_or("event"),
            &fields.0,
            fields.1,
            span.as_ref().map(|s| s.id().into_u64()),
            span.as_ref()
                .and_then(|s| s.parent())
                .map(|s| s.id().into_u64()),
        );
    }
    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        if let (Some(s), Some(span)) = (
            self.sink.read().ok().and_then(|s| s.upgrade()),
            ctx.span(&id),
        ) {
            if let Some(data) = span.extensions().get::<SpanData>() {
                s.record(
                    if data.failed { "ERROR" } else { "INFO" },
                    span.metadata().target(),
                    span.name(),
                    &format!(
                        "{} {}",
                        if data.failed { "failed" } else { "completed" },
                        data.fields.0
                    ),
                    Some(data.started.elapsed().as_secs_f64() * 1000.),
                    Some(id.into_u64()),
                    data.parent,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracing_captures_nested_errors_fields_and_elapsed_time() {
        let s = Arc::new(DebugService::new(100));
        let subscriber = tracing_subscriber::registry().with(Capture {
            sink: Arc::new(RwLock::new(Arc::downgrade(&s))),
        });
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!(target:"photag_plugin_test","operation",file_id=42);
            let _guard = span.enter();
            {
                let child = tracing::info_span!("child");
                let _guard = child.enter();
                tracing::error!("actual failure");
            }
            tracing::info!(duration_ms = 12.5, "worker elapsed");
        });
        let p = s.query(&crate::Filter::default());
        assert!(p.records.iter().any(|r| r.action == "operation"
            && r.duration_ms.is_some()
            && r.message.contains("file_id=42")));
        assert!(p.records.iter().any(|r| r.action == "child"
            && r.level == "ERROR"
            && r.duration_ms.is_some()
            && r.parent_span_id.is_some()));
        assert!(p.records.iter().any(|r| r.duration_ms == Some(12.5)));
        assert!(
            p.records
                .iter()
                .all(|r| !r.time.is_empty() && r.elapsed_ms >= 0.)
        );
    }
}
