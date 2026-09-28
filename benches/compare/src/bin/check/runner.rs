use crate::worker::{PANIC, PARSED, REJECTED};
use mail_parser_compare::{
    corpus::Corpus,
    tally::Tally,
    workload::{Implementation, Workload},
};
use std::{
    env,
    error::Error,
    os::unix::process::ExitStatusExt,
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

pub type BoxError = Box<dyn Error + Send + Sync>;
type JobResult = Result<Vec<Outcome>, BoxError>;

#[derive(Debug, Clone, Copy)]
pub enum Outcome {
    Parsed(Tally),
    Rejected,
    Panic,
    Crash,
}

impl Outcome {
    pub fn tally(&self) -> Option<&Tally> {
        match self {
            Outcome::Parsed(tally) => Some(tally),
            Outcome::Rejected | Outcome::Panic | Outcome::Crash => None,
        }
    }
}

pub struct Job {
    pub workload: Workload,
    pub corpus: &'static Corpus,
    pub implementation: Implementation,
}

pub struct Run {
    pub job: Job,
    pub outcomes: Vec<Outcome>,
}

impl Job {
    fn run(&self) -> JobResult {
        let total = self.corpus.samples.len();
        let executable = env::current_exe()?;
        let mut outcomes = Vec::with_capacity(total);
        while outcomes.len() < total {
            let output = Command::new(&executable)
                .arg("--worker")
                .arg(self.implementation.name())
                .arg(self.workload.name())
                .arg(&self.corpus.name)
                .arg(outcomes.len().to_string())
                .stdin(Stdio::null())
                .stderr(Stdio::inherit())
                .output()?;
            for line in String::from_utf8(output.stdout)?.lines() {
                let (index, outcome) = parse_line(line)?;
                if index != outcomes.len() {
                    return Err(format!("worker skipped to message {index}").into());
                }
                outcomes.push(outcome);
            }
            if output.status.success() {
                if outcomes.len() < total {
                    return Err(format!(
                        "worker for {} {} {} stopped after {} of {total} messages",
                        self.implementation.name(),
                        self.workload.name(),
                        self.corpus.name,
                        outcomes.len()
                    )
                    .into());
                }
            } else if output.status.signal().is_some() {
                if outcomes.len() < total {
                    outcomes.push(Outcome::Crash);
                }
            } else {
                return Err(format!(
                    "worker for {} {} {} failed: {}",
                    self.implementation.name(),
                    self.workload.name(),
                    self.corpus.name,
                    output.status
                )
                .into());
            }
        }
        Ok(outcomes)
    }
}

fn parse_line(line: &str) -> Result<(usize, Outcome), BoxError> {
    let mut columns = line.split('\t');
    let index = columns.next().ok_or("empty worker line")?.parse()?;
    let outcome = match columns.next() {
        Some(PARSED) => {
            let mut fields = [0u64; 12];
            let mut values = columns.next().ok_or("missing tally")?.split(' ');
            for field in &mut fields {
                *field = values.next().ok_or("short tally")?.parse()?;
            }
            Outcome::Parsed(Tally::from_fields(fields))
        }
        Some(REJECTED) => Outcome::Rejected,
        Some(PANIC) => Outcome::Panic,
        _ => return Err(format!("bad worker line {line}").into()),
    };
    Ok((index, outcome))
}

pub fn run_all(jobs: Vec<Job>, threads: usize) -> Result<Vec<Run>, BoxError> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<JobResult>>> = Mutex::new(jobs.iter().map(|_| None).collect());
    thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(index) else {
                        break;
                    };
                    let result = job.run();
                    eprintln!(
                        "checked {} {} {}",
                        job.workload.name(),
                        job.corpus.name,
                        job.implementation.name()
                    );
                    if let Ok(mut results) = results.lock()
                        && let Some(slot) = results.get_mut(index)
                    {
                        *slot = Some(result);
                    }
                }
            });
        }
    });
    let results = results.into_inner().map_err(|_| "result lock poisoned")?;
    jobs.into_iter()
        .zip(results)
        .map(|(job, result)| {
            let outcomes = result.ok_or("job did not run")??;
            Ok(Run { job, outcomes })
        })
        .collect()
}
