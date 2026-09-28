pub mod corpus;
pub mod ffi;
pub mod impls;
pub mod tally;
pub mod workload;

use corpus::Corpus;

pub fn bench_corpora() -> Vec<&'static Corpus> {
    let corpora = corpus::select(corpus::BENCH);
    for name in missing_bench_corpora(&corpora) {
        eprintln!(
            "warning: corpus {name} is not available (run scripts/fetch-corpora.sh or set {})",
            corpus::CORPORA_ENV
        );
    }
    corpora
}

pub fn missing_bench_corpora(corpora: &[&Corpus]) -> impl Iterator<Item = &'static str> {
    corpus::BENCH
        .iter()
        .copied()
        .filter(|name| !corpora.iter().any(|corpus| corpus.name == *name))
}
