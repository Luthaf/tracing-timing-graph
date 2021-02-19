use tracing_timing_graph::SpanTimingLayer;
use tracing_subscriber::{prelude::*, registry::Registry};

#[tracing::instrument]
fn run_computation(max: u64) {
    for i in 0..max {
        compute(i)
    }

    {
        // manual span
        let span = tracing::span!(tracing::Level::INFO, "another span");
        let _guard = span.enter();
        details::bottom_5us();
    }

    for _ in 0..(max * max) {
        details::bottom_5us();
    }
}

#[tracing::instrument]
pub fn compute(count: u64) {
    for _ in 0..count {
        details::bottom_5us();
    }
}

mod details {
    #[tracing::instrument]
    pub fn bottom_5us() {
        std::thread::sleep(std::time::Duration::from_micros(5));
    }
}

#[tracing::instrument]
fn run_other_5ms() {
    std::thread::sleep(std::time::Duration::from_millis(5));
}

fn main() {
    let span_timer = SpanTimingLayer::new();
    let graph = span_timer.graph();
    let subscriber = Registry::default().with(span_timer);

    tracing::subscriber::set_global_default(subscriber).unwrap();

    run_other_5ms();
    run_computation(10);

    let graph = graph.lock(); // .unwrap();

    println!("{}", graph.as_dot());
    println!("{}", graph.as_json());
    println!("{}", graph.as_table());
}
