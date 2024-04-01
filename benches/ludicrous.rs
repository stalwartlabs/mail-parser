use criterion::{BatchSize, Bencher, black_box, Criterion};
use rand::Rng;
use rand::rngs::OsRng;


static HDR_HASH: &[u8] = &[
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 0, 20, 5, 0, 0, 25, 0, 5, 20, 73, 25, 25, 30, 10, 10, 5, 73, 0, 0, 15, 73, 73, 73, 73, 20,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
];

/// Confirm whether ludicrous mode makes an impact.
pub fn ludicrous_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ludicrous");
    group.sample_size(10);
    group.bench_function("with", bench_with);
    group.bench_function("without", bench_without);
}

/// With unsafe ludicrous mode code.
fn bench_with(b: &mut Bencher) {
    b.iter_batched(|| {
        OsRng.gen()
    }, |rand: u8| {
        black_box({
            unsafe {
                *HDR_HASH.get_unchecked(rand as usize)
            }
        })
    }, BatchSize::SmallInput);
}

/// Fully safe code.
fn bench_without(b: &mut Bencher) {
    b.iter_batched(|| {
        OsRng.gen()
    }, |rand: u8| {
        black_box({
            HDR_HASH[rand as usize]
        })
    }, BatchSize::SmallInput);
}
