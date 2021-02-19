use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::{LookupSpan, SpanRef};

use parking_lot::Mutex;
use quanta::Clock;

use std::sync::Arc;
use std::time::Duration;

use crate::SpanGraph;

/// Extension to store timing data on spans
struct SpanTimingExtension {
    /// Last start of this span, as given by `quanta::Clock::start()`
    start: Option<u64>,
    /// Total elapsed time on this span, counting all enter/exit pairs
    elapsed: Duration,
}

impl SpanTimingExtension {
    fn new() -> SpanTimingExtension {
        SpanTimingExtension {
            start: None,
            elapsed: Duration::new(0, 0),
        }
    }
}

/// `tracing_subscriber` Layer that add timing information to spans,
/// accounting for the full span graph.
pub struct SpanTimingLayer {
    clock: Clock,
    timings: Arc<Mutex<SpanGraph>>,
}

impl SpanTimingLayer {
    /// Create a new empty `SpanTimingLayer`
    pub fn new() -> SpanTimingLayer {
        SpanTimingLayer {
            clock: Clock::new(),
            timings: Arc::new(Mutex::new(SpanGraph::new())),
        }
    }

    /// Get a reference to the span graph in this layer
    pub fn graph(&self) -> Arc<Mutex<SpanGraph>> {
        Arc::clone(&self.timings)
    }
}

impl<S> Layer<S> for SpanTimingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn new_span(&self, _: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("failed to get newly created span");
        let mut extensions = span.extensions_mut();
        extensions.insert(SpanTimingExtension::new());
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("on_enter: failed to get span");
        let mut extensions = span.extensions_mut();
        let mut timing = extensions
            .get_mut::<SpanTimingExtension>()
            .expect("on_enter: failed to get SpanTimingExtension");
        debug_assert!(timing.start.is_none());
        timing.start = Some(self.clock.start());
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("on_exit: failed to get span");
        let mut extensions = span.extensions_mut();
        let mut timing = extensions
            .get_mut::<SpanTimingExtension>()
            .expect("on_exit: failed to get SpanTimingExtension");

        let end = self.clock.end();
        timing.elapsed += self.clock.delta(
            timing.start.expect("on_exit: failed to get start time"),
            end,
        );
        timing.start = None;
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("on_close: failed to get span");
        let extensions = span.extensions();
        let timing = extensions
            .get::<SpanTimingExtension>()
            .expect("on_close: failed to get SpanTimingExtension");
        debug_assert!(timing.start.is_none());

        let mut graph = self.timings.lock(); // .expect("poisoned lock");

        let full_name = |span: &SpanRef<'_, S>| {
            let mut name = if let Some(path) = span.metadata().module_path() {
                path.to_string()
            } else {
                span.metadata().target().to_string()
            };
            name += "::";

            if span.name().contains(' ') {
                name += "{";
                name += span.name();
                name += "}";
            } else {
                name += span.name();
            }

            return name;
        };

        // create the parent first to ensure it has a lower node id than the
        // child. This makes the final output looks a bit better
        let parent = span
            .parent()
            .map(|id| graph.find_or_create(&full_name(&id)));

        let current = graph.find_or_create(&full_name(&span));
        graph.increase_timing(current, timing.elapsed);

        if let Some(parent) = parent {
            graph.increase_call_count(parent, current);
        }
    }
}
