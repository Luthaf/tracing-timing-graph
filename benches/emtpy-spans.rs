use tracing_timing_graph::SpanTimingLayer;
use tracing_subscriber::{prelude::*, Registry};

use criterion::{Criterion, criterion_group, criterion_main};

use once_cell::sync::Lazy;

static SUBSCRIBER: Lazy<()> = Lazy::new(|| {
    let subscriber = Registry::default().with(SpanTimingLayer::new());
    tracing::subscriber::set_global_default(subscriber).unwrap();
});

#[tracing::instrument]
fn compute_stuff() {
    let _ = tracing::span!(tracing::Level::INFO, "empty").enter();
    let _ = tracing::span!(tracing::Level::INFO, "another").enter();
}

fn empty_spans(c: &mut Criterion) {
    Lazy::force(&SUBSCRIBER);
    c.bench_function("empty spans", |b| b.iter(compute_stuff));
}

criterion_group!(benches, empty_spans);
criterion_main!(benches);
