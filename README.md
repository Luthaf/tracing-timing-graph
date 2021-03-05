# Tracing timing graph

This has been superseeded for my use by https://github.com/Luthaf/time-graph. 
I can transfer this code to you if you are interested in the integration with 
tracing ecosystem.

----

This crate provides a simple way of extracting the number of time a given
function (or spans inside functions) have been called, how much time have been
spent in each function/span, and record the full "call-graph" between
functions/spans. The indented use case is to extract simple profiling data from
actual runs of a software. Importantly, this crate does not consider different
invocation of the same function/span separately, but instead group all
invocation of functions/span together.

This crate is built on top of the [`tracing`](https://tracing.rs/tracing/) and
[`tracing-subscriber`](https://tracing.rs/tracing_subscriber/index.html) crates.
