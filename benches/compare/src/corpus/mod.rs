mod synthetic;

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const CORPORA_ENV: &str = "MAIL_PARSER_CORPORA";
pub const ENRON_MAILDIR: &str = "enron/maildir";
pub const STALWART_SMTP: &str = "stalwart-smtp";
const SIEVE_LABEL: &str = "sieve";
const ENRON_PER_USER: usize = 20;
const TESTSUITE_SUITES: [&str; 4] = ["rfc", "legacy", "thirdparty", "malformed"];
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub const BENCH: &[&str] = &[
    "testsuite-lf",
    "testsuite-crlf",
    "enron-crlf",
    "attachments-crlf",
    "newsletter-crlf",
    "modern-headers-crlf",
    "forwarded-crlf",
    "dashes-crlf",
    "plain-large-crlf",
    "attachments-lf",
    "newsletter-lf",
];

static TESTSUITE: OnceLock<Vec<Corpus>> = OnceLock::new();
static ENRON: OnceLock<Vec<Corpus>> = OnceLock::new();
static SYNTHETIC: OnceLock<Vec<Corpus>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub const ALL: [LineEnding; 2] = [LineEnding::Lf, LineEnding::Crlf];

    pub fn suffix(self) -> &'static str {
        match self {
            LineEnding::Lf => "lf",
            LineEnding::Crlf => "crlf",
        }
    }

    pub fn apply(self, bytes: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(bytes.len() + bytes.len() / 32);
        match self {
            LineEnding::Lf => result.extend(bytes.iter().filter(|&&byte| byte != b'\r')),
            LineEnding::Crlf => {
                let mut last = 0;
                for &byte in bytes {
                    if byte == b'\n' && last != b'\r' {
                        result.push(b'\r');
                    }
                    result.push(byte);
                    last = byte;
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Corpus {
    pub name: String,
    pub samples: Vec<Sample>,
}

impl Corpus {
    pub fn total_bytes(&self) -> u64 {
        self.samples.iter().map(|s| s.bytes.len() as u64).sum()
    }

    pub fn mib(&self) -> f64 {
        self.total_bytes() as f64 / f64::from(1u32 << 20)
    }

    pub fn digest(&self) -> u64 {
        self.samples.iter().fold(FNV_OFFSET, |hash, sample| {
            sample
                .bytes
                .iter()
                .copied()
                .chain((sample.bytes.len() as u64).to_le_bytes())
                .fold(hash, |hash, byte| {
                    (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
                })
        })
    }

    fn with_line_ending(name: &str, line_ending: LineEnding, samples: &[Sample]) -> Self {
        Corpus {
            name: format!("{name}-{}", line_ending.suffix()),
            samples: samples
                .iter()
                .map(|sample| Sample {
                    name: sample.name.clone(),
                    bytes: line_ending.apply(&sample.bytes),
                })
                .collect(),
        }
    }

    fn both_line_endings(name: &str, samples: &[Sample]) -> impl Iterator<Item = Corpus> {
        LineEnding::ALL
            .into_iter()
            .map(move |line_ending| Corpus::with_line_ending(name, line_ending, samples))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Testsuite,
    Enron,
    Synthetic,
}

impl Family {
    fn of(name: &str) -> Option<Family> {
        let (base, ending) = name.rsplit_once('-')?;
        if !LineEnding::ALL.iter().any(|le| le.suffix() == ending) {
            return None;
        }
        match base {
            "testsuite" => Some(Family::Testsuite),
            "enron" => Some(Family::Enron),
            _ if synthetic::NAMES.contains(&base) => Some(Family::Synthetic),
            _ => None,
        }
    }

    fn corpora(self) -> &'static [Corpus] {
        match self {
            Family::Testsuite => TESTSUITE.get_or_init(load_testsuite),
            Family::Enron => ENRON.get_or_init(load_enron),
            Family::Synthetic => SYNTHETIC.get_or_init(load_synthetic),
        }
    }
}

pub fn get(name: &str) -> Option<&'static Corpus> {
    Family::of(name)?
        .corpora()
        .iter()
        .find(|corpus| corpus.name == name)
}

pub fn select(names: &[&str]) -> Vec<&'static Corpus> {
    names.iter().filter_map(|name| get(name)).collect()
}

pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

pub fn corpora_root() -> PathBuf {
    env::var_os(CORPORA_ENV).map_or_else(
        || repository_root().join("target").join("corpora"),
        PathBuf::from,
    )
}

pub fn corpus_dir(name: &str) -> PathBuf {
    let dir = corpora_root().join(name);
    if !dir.is_dir() {
        eprintln!(
            "corpus {name} not found in {}: skipped (run scripts/fetch-corpora.sh or set {CORPORA_ENV})",
            dir.display()
        );
    }
    dir
}

fn resources_dir() -> PathBuf {
    repository_root().join("resources")
}

fn load_testsuite() -> Vec<Corpus> {
    let samples = testsuite_samples();
    Corpus::both_line_endings("testsuite", &samples).collect()
}

fn load_enron() -> Vec<Corpus> {
    let samples = enron_samples(&corpus_dir(ENRON_MAILDIR), ENRON_PER_USER);
    if samples.is_empty() {
        return Vec::new();
    }
    vec![Corpus {
        name: "enron-crlf".to_string(),
        samples,
    }]
}

fn load_synthetic() -> Vec<Corpus> {
    let mut corpora = Vec::with_capacity(synthetic::NAMES.len() * LineEnding::ALL.len());
    for (name, samples) in synthetic::NAMES.into_iter().zip(synthetic::generate()) {
        corpora.extend(Corpus::both_line_endings(name, &samples));
    }
    corpora
}

fn read_dir_sorted(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    paths
}

fn load_sample(path: &Path, root: &Path, prefix: &str) -> Option<Sample> {
    let relative = path.strip_prefix(root).unwrap_or(path).display();
    let name = if prefix.is_empty() {
        relative.to_string()
    } else {
        format!("{prefix}/{relative}")
    };
    fs::read(path).ok().map(|bytes| Sample { name, bytes })
}

fn testsuite_samples() -> Vec<Sample> {
    let resources = resources_dir();
    let eml = resources.join("eml");
    let suites = TESTSUITE_SUITES
        .iter()
        .flat_map(|suite| read_dir_sorted(&eml.join(suite)))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "eml"))
        .filter_map(|path| load_sample(&path, &eml, ""));
    let extra = [
        (STALWART_SMTP, corpus_dir(STALWART_SMTP)),
        (SIEVE_LABEL, resources.join(SIEVE_LABEL)),
    ]
    .into_iter()
    .flat_map(|(label, root)| {
        read_dir_sorted(&root)
            .into_iter()
            .filter(|path| path.is_file())
            .filter_map(move |path| load_sample(&path, &root, label))
    });
    suites.chain(extra).collect()
}

fn enron_samples(root: &Path, per_user: usize) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(per_user * 160);
    for user in read_dir_sorted(root) {
        let mut stack = vec![user];
        let mut taken = 0;
        while let Some(dir) = stack.pop() {
            for entry in read_dir_sorted(&dir) {
                if entry.is_dir() {
                    stack.push(entry);
                } else if taken < per_user {
                    samples.extend(load_sample(&entry, root, ""));
                    taken += 1;
                }
            }
            if taken >= per_user {
                break;
            }
        }
    }
    samples
}
