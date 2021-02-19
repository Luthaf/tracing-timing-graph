//! This crate provides a simple way of extracting the number of time a given
//! function (or spans inside functions) have been called, and how much time
//! have been spent in each function/span.
//!
//! It is built on top of the [`tracing`](https://tracing.rs/tracing/) and
//! [`tracing-subscriber`](https://tracing.rs/tracing_subscriber/index.html)
//! crates.
//!
//! Importantly, this crate does not consider different invocation of the same
//! function/span separately, but instead group all invocation of functions/span
//! together.

#![allow(clippy::needless_return, clippy::redundant_field_names, clippy::new_without_default)]

mod graph;
pub use self::graph::{SpanGraph, SpanIndex, SpanTiming};

mod layer;
pub use self::layer::SpanTimingLayer;
