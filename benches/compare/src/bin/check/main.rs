mod report;
mod runner;
mod worker;

use mail_parser_compare::{
    bench_corpora,
    corpus::{self, Corpus},
    workload::{Implementation, Workload},
};
use runner::{BoxError, Job};
use std::{env, fs, path::PathBuf};

const DEFAULT_THREADS: usize = 4;

#[derive(Default)]
struct Options {
    corpora: Vec<String>,
    implementations: Vec<Implementation>,
    excluded: Vec<Implementation>,
    workloads: Vec<Workload>,
    out: Option<PathBuf>,
    examples: usize,
    threads: Option<usize>,
}

impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Options, BoxError> {
        let mut options = Options::default();
        while let Some(arg) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--corpus" => options.corpora.push(value()?),
                "--implementation" => options.implementations.push(implementation(&value()?)?),
                "--exclude" => options.excluded.push(implementation(&value()?)?),
                "--workload" => {
                    let name = value()?;
                    options.workloads.push(
                        Workload::from_name(&name)
                            .ok_or_else(|| format!("unknown workload {name}"))?,
                    );
                }
                "--out" => options.out = Some(PathBuf::from(value()?)),
                "--examples" => options.examples = value()?.parse()?,
                "--jobs" => options.threads = Some(value()?.parse()?),
                _ => return Err(format!("unknown argument {arg}").into()),
            }
        }
        Ok(options)
    }

    fn corpora(&self) -> Result<Vec<&'static Corpus>, BoxError> {
        if self.corpora.is_empty() {
            return Ok(bench_corpora());
        }
        self.corpora
            .iter()
            .map(|name| corpus::get(name).ok_or_else(|| format!("unknown corpus {name}").into()))
            .collect()
    }

    fn implementations(&self) -> Vec<Implementation> {
        let mut selected = if self.implementations.is_empty() {
            Implementation::ALL.to_vec()
        } else {
            self.implementations.clone()
        };
        selected.retain(|implementation| !self.excluded.contains(implementation));
        if !selected.contains(&Implementation::REFERENCE) {
            selected.insert(0, Implementation::REFERENCE);
        }
        selected
    }

    fn workloads(&self) -> Vec<Workload> {
        if self.workloads.is_empty() {
            Workload::ALL.to_vec()
        } else {
            self.workloads.clone()
        }
    }
}

fn implementation(name: &str) -> Result<Implementation, BoxError> {
    Implementation::from_name(name).ok_or_else(|| format!("unknown implementation {name}").into())
}

fn main() -> Result<(), BoxError> {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(worker_args) = args.strip_prefix(&["--worker".to_string()]) {
        return worker::main(worker_args).map_err(|error| error.to_string().into());
    }
    let options = Options::parse(args.into_iter())?;
    let corpora = options.corpora()?;
    let implementations = options.implementations();
    let workloads = options.workloads();
    let mut jobs = Vec::with_capacity(corpora.len() * implementations.len() * workloads.len());
    for &workload in &workloads {
        for &corpus in &corpora {
            jobs.extend(implementations.iter().map(|&implementation| Job {
                workload,
                corpus,
                implementation,
            }));
        }
    }
    let threads = options.threads.unwrap_or(DEFAULT_THREADS);
    let runs = runner::run_all(jobs, threads)?;
    let mut out = String::with_capacity(64 * 1024);
    out.push_str("# Equivalence check\n\n");
    report::corpora(&mut out, &corpora, options.corpora.is_empty());
    for &workload in &workloads {
        report::workload(&mut out, workload, &runs, &corpora);
    }
    report::examples(&mut out, &runs, options.examples);
    print!("{out}");
    if let Some(path) = options.out {
        fs::write(&path, &out)?;
        eprintln!("written to {}", path.display());
    }
    Ok(())
}
