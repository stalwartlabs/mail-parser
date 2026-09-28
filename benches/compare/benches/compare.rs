use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use mail_parser_compare::{
    bench_corpora,
    workload::{Implementation, Parsers, Workload},
};
use std::{hint::black_box, time::Duration};

const WARM_UP: Duration = Duration::from_millis(500);
const MEASUREMENT: Duration = Duration::from_secs(3);
const SAMPLES: usize = 30;

fn compare(c: &mut Criterion) {
    let parsers = Parsers::default();
    for corpus in bench_corpora() {
        for workload in Workload::ALL {
            let mut group = c.benchmark_group(format!("{}/{}", workload.name(), corpus.name));
            group.throughput(Throughput::Bytes(corpus.total_bytes()));
            for implementation in Implementation::ALL {
                group.bench_function(implementation.name(), |b| {
                    b.iter(|| parsers.run_corpus(implementation, workload, black_box(corpus)))
                });
            }
            group.finish();
        }
    }
}

fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(WARM_UP)
        .measurement_time(MEASUREMENT)
        .sample_size(SAMPLES)
}

criterion_group! {
    name = benches;
    config = config();
    targets = compare
}
criterion_main!(benches);
