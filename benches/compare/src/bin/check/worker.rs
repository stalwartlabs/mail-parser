use mail_parser_compare::{
    corpus,
    tally::Tally,
    workload::{Implementation, Parsers, Workload},
};
use std::{
    error::Error,
    io::{self, Write},
    panic::{self, AssertUnwindSafe},
};

pub const REJECTED: &str = "rejected";
pub const PANIC: &str = "panic";
pub const PARSED: &str = "parsed";

pub fn main(args: &[String]) -> Result<(), Box<dyn Error>> {
    let [implementation, workload, corpus_name, start] = args else {
        return Err("usage: check --worker <implementation> <workload> <corpus> <start>".into());
    };
    let implementation = Implementation::from_name(implementation)
        .ok_or_else(|| format!("unknown implementation {implementation}"))?;
    let workload =
        Workload::from_name(workload).ok_or_else(|| format!("unknown workload {workload}"))?;
    let corpus = corpus::get(corpus_name).ok_or_else(|| format!("unknown corpus {corpus_name}"))?;
    let start: usize = start.parse()?;
    let parsers = Parsers::default();
    panic::set_hook(Box::new(|_| {}));
    let mut out = io::stdout().lock();
    for (index, sample) in corpus.samples.iter().enumerate().skip(start) {
        let mut tally = Tally::default();
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            parsers.run(implementation, workload, &sample.bytes, &mut tally)
        }));
        match outcome {
            Ok(true) => {
                write!(out, "{index}\t{PARSED}\t")?;
                for (position, field) in tally.fields().iter().enumerate() {
                    let separator = if position == 0 { "" } else { " " };
                    write!(out, "{separator}{field}")?;
                }
                writeln!(out)?;
            }
            Ok(false) => writeln!(out, "{index}\t{REJECTED}")?,
            Err(_) => writeln!(out, "{index}\t{PANIC}")?,
        }
        out.flush()?;
    }
    Ok(())
}
