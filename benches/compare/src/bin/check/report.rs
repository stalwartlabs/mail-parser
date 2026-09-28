use crate::runner::{Outcome, Run};
use mail_parser_compare::{
    corpus::{self, Corpus},
    missing_bench_corpora,
    tally::Tally,
    workload::{Implementation, Workload},
};
use std::fmt::Write as _;

const MIB: f64 = 1_048_576.0;

#[derive(Default)]
struct Summary {
    parsed: u64,
    rejected: u64,
    panics: u64,
    crashes: u64,
    differing: u64,
    total: Tally,
}

impl Summary {
    fn new(run: &Run, reference: Option<&Run>) -> Summary {
        let mut summary = Summary::default();
        for outcome in &run.outcomes {
            match outcome {
                Outcome::Parsed(tally) => {
                    summary.parsed += 1;
                    summary.total += *tally;
                }
                Outcome::Rejected => summary.rejected += 1,
                Outcome::Panic => summary.panics += 1,
                Outcome::Crash => summary.crashes += 1,
            }
        }
        summary.differing =
            reference.map_or(0, |reference| differences(run, reference).count() as u64);
        summary
    }
}

fn key(workload: Workload, outcome: &Outcome) -> Option<[u64; 4]> {
    let tally = outcome.tally()?;
    Some(match workload {
        Workload::Structure => [tally.messages, tally.parts, 0, 0],
        Workload::Headers => [tally.subject, tally.from, tally.date, tally.message_id],
        Workload::Full => [tally.messages, tally.leaves, tally.decoded_bytes, 0],
    })
}

fn differences<'a>(
    run: &'a Run,
    reference: &'a Run,
) -> impl Iterator<Item = (usize, &'a Outcome, &'a Outcome)> {
    let workload = run.job.workload;
    run.outcomes
        .iter()
        .zip(&reference.outcomes)
        .enumerate()
        .filter(move |(_, (outcome, expected))| key(workload, outcome) != key(workload, expected))
        .map(|(index, (outcome, expected))| (index, outcome, expected))
}

fn delta(value: u64, reference: Option<u64>) -> String {
    match reference {
        Some(reference) if reference != value => {
            format!("{value} ({:+})", value as i128 - reference as i128)
        }
        _ => value.to_string(),
    }
}

fn percent(value: u64, reference: Option<u64>) -> String {
    match reference {
        Some(reference) if reference > 0 && reference != value => format!(
            "{:+.2}%",
            (value as f64 - reference as f64) / reference as f64 * 100.0
        ),
        Some(_) => "=".to_string(),
        None => String::new(),
    }
}

fn describe(workload: Workload, outcome: &Outcome) -> String {
    match (outcome, key(workload, outcome)) {
        (_, Some([a, b, c, d])) => match workload {
            Workload::Structure => format!("{a} messages, {b} parts"),
            Workload::Headers => format!("subject {a}, from {b}, date {c}, message-id {d}"),
            Workload::Full => format!("{a} messages, {b} leaves, {c} bytes"),
        },
        (Outcome::Rejected, None) => "rejected".to_string(),
        (Outcome::Panic, None) => "panic".to_string(),
        (Outcome::Crash, None) => "crash".to_string(),
        (Outcome::Parsed(_), None) => "parsed".to_string(),
    }
}

pub fn corpora(out: &mut String, corpora: &[&Corpus], full_set: bool) {
    if full_set {
        for name in missing_bench_corpora(corpora) {
            let _ = writeln!(
                out,
                "**Missing corpus: {name}** (run `scripts/fetch-corpora.sh` or point `{}` at the corpora directory).\n",
                corpus::CORPORA_ENV
            );
        }
    }
    out.push_str("| corpus | messages | bytes | MiB | FNV-1a digest |\n|---|---:|---:|---:|---|\n");
    for corpus in corpora {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.2} | `{:016x}` |",
            corpus.name,
            corpus.samples.len(),
            corpus.total_bytes(),
            corpus.mib(),
            corpus.digest()
        );
    }
    out.push('\n');
}

pub fn workload(out: &mut String, workload: Workload, runs: &[Run], corpora: &[&Corpus]) {
    let _ = writeln!(out, "## {}\n", workload.name());
    out.push_str("| corpus | implementation | parsed | rejected | panics | crashes |");
    match workload {
        Workload::Structure => out.push_str(" messages | parts | differing messages |\n|---|---|---:|---:|---:|---:|---:|---:|---:|\n"),
        Workload::Headers => out.push_str(" subject | from | date | message-id | differing messages |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n"),
        Workload::Full => out.push_str(" messages | leaves | text leaves | decoded MiB | decoded vs 1.0 | failures | differing messages |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n"),
    }
    for corpus in corpora {
        let selected: Vec<&Run> = runs
            .iter()
            .filter(|run| run.job.workload == workload && run.job.corpus.name == corpus.name)
            .collect();
        let reference = selected
            .iter()
            .find(|run| run.job.implementation == Implementation::REFERENCE)
            .copied();
        let reference_summary = reference.map(|run| Summary::new(run, None));
        let base = |field: fn(&Summary) -> u64| reference_summary.as_ref().map(field);
        for run in &selected {
            let summary = Summary::new(run, reference);
            let _ = write!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                corpus.name,
                run.job.implementation.name(),
                summary.parsed,
                summary.rejected,
                summary.panics,
                summary.crashes
            );
            let total = &summary.total;
            let _ = match workload {
                Workload::Structure => writeln!(
                    out,
                    " {} | {} | {} |",
                    delta(total.messages, base(|s| s.total.messages)),
                    delta(total.parts, base(|s| s.total.parts)),
                    summary.differing
                ),
                Workload::Headers => writeln!(
                    out,
                    " {} | {} | {} | {} | {} |",
                    delta(total.subject, base(|s| s.total.subject)),
                    delta(total.from, base(|s| s.total.from)),
                    delta(total.date, base(|s| s.total.date)),
                    delta(total.message_id, base(|s| s.total.message_id)),
                    summary.differing
                ),
                Workload::Full => writeln!(
                    out,
                    " {} | {} | {} | {:.2} | {} | {} | {} |",
                    delta(total.messages, base(|s| s.total.messages)),
                    delta(total.leaves, base(|s| s.total.leaves)),
                    delta(total.text_leaves, base(|s| s.total.text_leaves)),
                    total.decoded_bytes as f64 / MIB,
                    percent(total.decoded_bytes, base(|s| s.total.decoded_bytes)),
                    total.failures,
                    summary.differing
                ),
            };
        }
    }
    out.push('\n');
}

pub fn examples(out: &mut String, runs: &[Run], limit: usize) {
    if limit == 0 {
        return;
    }
    let _ = writeln!(out, "## Differing messages (first {limit} per cell)\n");
    for run in runs {
        if run.job.implementation == Implementation::REFERENCE {
            continue;
        }
        let Some(reference) = runs.iter().find(|candidate| {
            candidate.job.implementation == Implementation::REFERENCE
                && candidate.job.workload == run.job.workload
                && candidate.job.corpus.name == run.job.corpus.name
        }) else {
            continue;
        };
        let workload = run.job.workload;
        for (index, outcome, expected) in differences(run, reference).take(limit) {
            let name = run
                .job
                .corpus
                .samples
                .get(index)
                .map_or("?", |sample| sample.name.as_str());
            let _ = writeln!(
                out,
                "- {} {} {} `{name}`: {} (1.0: {})",
                workload.name(),
                run.job.corpus.name,
                run.job.implementation.name(),
                describe(workload, outcome),
                describe(workload, expected)
            );
        }
    }
    out.push('\n');
}
