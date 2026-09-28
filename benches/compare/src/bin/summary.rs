use mail_parser_compare::{
    corpus,
    workload::{Implementation, Workload},
};
use serde_json::Value;
use std::{
    env,
    error::Error,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

struct Entry {
    workload: String,
    corpus: String,
    implementation: String,
    median_ns: f64,
    bytes: f64,
}

impl Entry {
    fn mib_per_second(&self) -> f64 {
        self.bytes / self.median_ns * 1e9 / f64::from(1u32 << 20)
    }

    fn read(dir: &Path) -> Option<Entry> {
        let benchmark: Value =
            serde_json::from_slice(&fs::read(dir.join("benchmark.json")).ok()?).ok()?;
        let estimates: Value =
            serde_json::from_slice(&fs::read(dir.join("estimates.json")).ok()?).ok()?;
        let (workload, corpus) = benchmark.get("group_id")?.as_str()?.split_once('/')?;
        Workload::from_name(workload)?;
        let throughput = benchmark.get("throughput")?;
        let bytes = throughput
            .get("Bytes")
            .or_else(|| throughput.get("BytesDecimal"))?
            .as_f64()?;
        Some(Entry {
            workload: workload.to_string(),
            corpus: corpus.to_string(),
            implementation: benchmark.get("function_id")?.as_str()?.to_string(),
            median_ns: estimates.get("median")?.get("point_estimate")?.as_f64()?,
            bytes,
        })
    }
}

fn criterion_dir() -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("target"),
            PathBuf::from,
        )
        .join("criterion")
}

fn collect(root: &Path) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(children) = fs::read_dir(&dir) else {
            continue;
        };
        for child in children.flatten().map(|child| child.path()) {
            if !child.is_dir() {
                continue;
            }
            if child.file_name().is_some_and(|name| name == "new") {
                entries.extend(Entry::read(&child));
            } else {
                stack.push(child);
            }
        }
    }
    entries
}

fn position<T: PartialEq>(items: &[T], item: &T) -> usize {
    items
        .iter()
        .position(|candidate| candidate == item)
        .unwrap_or(items.len())
}

struct Cell<'a> {
    workload: &'a str,
    corpus: &'a str,
    speeds: Vec<(&'a str, Vec<f64>)>,
}

impl Cell<'_> {
    fn speeds(&self, implementation: &str) -> Option<&[f64]> {
        self.speeds
            .iter()
            .find(|(name, _)| *name == implementation)
            .map(|(_, speeds)| speeds.as_slice())
    }

    fn median(&self, implementation: &str) -> Option<f64> {
        self.speeds(implementation).and_then(median)
    }

    fn spread(&self, implementation: &str) -> Option<f64> {
        let speeds = self.speeds(implementation)?;
        let median = median(speeds)?;
        let (min, max) = speeds
            .iter()
            .fold((f64::MAX, f64::MIN), |(min, max), &speed| {
                (min.min(speed), max.max(speed))
            });
        Some((max - min) / median * 100.0)
    }
}

fn median(speeds: &[f64]) -> Option<f64> {
    let mut sorted = speeds.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    match sorted.len() {
        0 => None,
        len if len % 2 == 1 => sorted.get(middle).copied(),
        _ => Some((sorted.get(middle - 1)? + sorted.get(middle)?) / 2.0),
    }
}

fn cells<'a>(entries: &'a [Entry], implementations: &[&'a str]) -> Vec<Cell<'a>> {
    let mut cells: Vec<Cell<'a>> = Vec::new();
    for entry in entries {
        if !cells
            .iter()
            .any(|cell| cell.workload == entry.workload && cell.corpus == entry.corpus)
        {
            cells.push(Cell {
                workload: &entry.workload,
                corpus: &entry.corpus,
                speeds: implementations
                    .iter()
                    .map(|name| (*name, Vec::new()))
                    .collect(),
            });
        }
    }
    for entry in entries {
        let speeds = cells
            .iter_mut()
            .find(|cell| cell.workload == entry.workload && cell.corpus == entry.corpus)
            .and_then(|cell| {
                cell.speeds
                    .iter_mut()
                    .find(|(name, _)| *name == entry.implementation)
            });
        if let Some((_, speeds)) = speeds {
            speeds.push(entry.mib_per_second());
        }
    }
    cells
}

fn header(table: &mut String, columns: impl Iterator<Item = String>) {
    table.push_str("| workload | corpus |");
    let mut count = 0;
    for column in columns {
        let _ = write!(table, " {column} |");
        count += 1;
    }
    table.push_str("\n|---|---|");
    table.push_str(&"---:|".repeat(count));
    table.push('\n');
}

fn format_value(value: Option<f64>, format: impl Fn(f64) -> String) -> String {
    value.map_or_else(String::new, format)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut out = None;
    let mut roots = Vec::new();
    let mut excluded = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--exclude" => {
                let name = args.next().ok_or("--exclude needs an implementation")?;
                excluded.push(
                    Implementation::from_name(&name)
                        .ok_or_else(|| format!("unknown implementation {name}"))?,
                );
            }
            _ => roots.push(PathBuf::from(arg)),
        }
    }
    if roots.is_empty() {
        roots.push(criterion_dir());
    }
    let mut entries: Vec<Entry> = roots.iter().flat_map(|root| collect(root)).collect();
    if entries.is_empty() {
        return Err("no criterion results found".into());
    }
    let workloads = Workload::ALL.map(Workload::name);
    let order = |entry: &Entry| {
        (
            position(&workloads, &entry.workload.as_str()),
            position(corpus::BENCH, &entry.corpus.as_str()),
            entry.corpus.clone(),
        )
    };
    entries.sort_by_cached_key(order);
    let implementations: Vec<&str> = Implementation::ALL
        .iter()
        .filter(|implementation| !excluded.contains(implementation))
        .map(|implementation| implementation.name())
        .filter(|name| entries.iter().any(|entry| entry.implementation == *name))
        .collect();
    let reference = Implementation::REFERENCE.name();
    let others: Vec<&str> = implementations
        .iter()
        .copied()
        .filter(|name| *name != reference)
        .collect();
    let cells = cells(&entries, &implementations);
    let mut table = String::with_capacity(16384);
    if roots.len() > 1 {
        let _ = writeln!(
            table,
            "Median of {} runs: MiB/s, and mail-parser 1.0 divided by each other implementation.\n",
            roots.len()
        );
    }
    header(
        &mut table,
        implementations
            .iter()
            .map(|name| format!("{name} MiB/s"))
            .chain(others.iter().map(|name| format!("1.0 / {name}"))),
    );
    for cell in &cells {
        let _ = write!(table, "| {} | {} |", cell.workload, cell.corpus);
        for name in &implementations {
            let value = format_value(cell.median(name), |value| format!("{value:.0}"));
            let _ = write!(table, " {value} |");
        }
        let base = cell.median(reference);
        for name in &others {
            let ratio = format_value(base.zip(cell.median(name)).map(|(a, b)| a / b), |ratio| {
                format!("{ratio:.2}x")
            });
            let _ = write!(table, " {ratio} |");
        }
        table.push('\n');
    }
    if roots.len() > 1 {
        table.push_str("\nSpread of the runs, (max - min) / median:\n\n");
        header(
            &mut table,
            implementations.iter().map(|name| (*name).to_string()),
        );
        for cell in &cells {
            let _ = write!(table, "| {} | {} |", cell.workload, cell.corpus);
            for name in &implementations {
                let spread = format_value(cell.spread(name), |spread| format!("{spread:.1}%"));
                let _ = write!(table, " {spread} |");
            }
            table.push('\n');
        }
    }
    print!("{table}");
    if let Some(out) = out {
        fs::write(&out, &table)?;
        eprintln!("written to {}", out.display());
    }
    Ok(())
}
