use criterion::{criterion_group, criterion_main};

mod ludicrous;
mod parse;

// run benchmarks with: cargo bench --bench bench
criterion_group!(benches, ludicrous::ludicrous_bench, parse::parse_bench);
criterion_main!(benches);
