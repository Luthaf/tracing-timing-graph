use tracing_timing_graph::SpanTimingLayer;
use tracing_subscriber::{prelude::*, registry::Registry};

#[tracing::instrument]
fn function_a(repeat: bool) {
    std::thread::sleep(std::time::Duration::from_millis(1));
    if repeat {
        function_b();
    }
}

#[tracing::instrument]
fn function_b() {
    std::thread::sleep(std::time::Duration::from_millis(1));
    function_a(false);
}

#[tracing::instrument]
fn recursive(mut count: usize) {
    std::thread::sleep(std::time::Duration::from_millis(1));
    count -= 1;
    if count > 0 {
        recursive(count);
    }
}

fn main() {
    let span_timer = SpanTimingLayer::new();
    let graph = span_timer.graph();
    let subscriber = Registry::default().with(span_timer);

    tracing::subscriber::set_global_default(subscriber).unwrap();

    recursive(4);
    function_a(true);

    let graph = graph.lock();

    println!("{}", graph.as_dot());
    println!("{}", graph.as_json());
    println!("{}", graph.as_table());
}
