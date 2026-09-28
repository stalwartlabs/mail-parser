use crate::scan::{self, ByteSet, swar};
use mail_parser::scan::Kernel;
use std::sync::LazyLock;

const SHIFTS: usize = 64;
const SET_SHIFTS: usize = 16;
const SPAN: usize = 256;
const MAX_MEMBERS: usize = 16;
const ASCII_MASK: u8 = 0x7f;
const BELOW_LIMIT: u8 = 0x80;
const CLASS: u8 = 1;
const BITMAP_BYTES: usize = 16;
const BITS: usize = 8;
const HIGH_BIT: usize = 7;
const FIELD_STOPS: u8 = 0x21;

static SIMPLE_NAME: ByteSet = ByteSet::with_marks(b"\n\r,;<\"(:=", b"@");
static SIMPLE_QUOTED: ByteSet = ByteSet::new(b"\n\r\"\\=");
static SIMPLE_ADDRESS: ByteSet = ByteSet::new(b"\n\r>\\(=");
static TOKEN_STOPS: LazyLock<ByteSet> = LazyLock::new(token_stops);

type Scan = fn(Kernel, &[u8], usize) -> Option<usize>;

const SCANS: [(Scan, &str); 2] = [
    (Kernel::dash_line, "dash_line"),
    (Kernel::field_end, "field_end"),
];

pub fn report() {
    let public: Vec<_> = Kernel::available().map(Kernel::name).collect();
    let sets: Vec<_> = scan::Kernel::available().map(scan::Kernel::name).collect();
    eprintln!(
        "kernels: scan {}; set {}",
        public.join(", "),
        sets.join(", ")
    );
}

pub fn check(data: &[u8]) {
    variants(data, SHIFTS, scans);
    for set in [&SIMPLE_NAME, &SIMPLE_QUOTED, &SIMPLE_ADDRESS, &TOKEN_STOPS] {
        sets(set, data);
    }
    if let Some((custom, hay)) = custom_sets(data) {
        for set in &custom {
            sets(set, hay);
        }
    }
    let (low, high, limit) = match data {
        [low, high, limit, ..] => (
            low & ASCII_MASK,
            high & ASCII_MASK,
            limit % (BELOW_LIMIT + 1),
        ),
        _ => (b'A', b'Z', FIELD_STOPS),
    };
    for skip in 0..data.len().min(swar::WORD) {
        words(data.get(skip..).unwrap_or_default(), low, high, limit);
    }
}

fn variants(data: &[u8], shifts: usize, mut each: impl FnMut(&[u8])) {
    each(data);
    let len = data.len();
    for skip in 1..len.min(shifts) {
        each(data.get(skip..len.min(skip + SPAN)).unwrap_or_default());
    }
    for cut in 1..len.min(shifts) {
        let end = len - cut;
        let tail: Box<[u8]> =
            Box::from(data.get(end.saturating_sub(SPAN)..end).unwrap_or_default());
        each(&tail);
    }
}

fn scans(hay: &[u8]) {
    for kernel in Kernel::available() {
        for (scan, name) in SCANS {
            let mut from = 0;
            loop {
                let expected = scan(Kernel::SCALAR, hay, from);
                assert_eq!(
                    scan(kernel, hay, from),
                    expected,
                    "{} {name} from {from} over {} bytes",
                    kernel.name(),
                    hay.len()
                );
                match expected {
                    Some(hit) => from = hit + 1,
                    None => break,
                }
            }
            for from in [hay.len(), hay.len() + 1, usize::MAX] {
                assert_eq!(scan(kernel, hay, from), scan(Kernel::SCALAR, hay, from));
            }
        }
    }
}

fn token_stops() -> ByteSet {
    let classes: [u8; 256] = std::array::from_fn(|byte| {
        let byte = byte as u8;
        let token = byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`{|}~".contains(&byte);
        if token || !byte.is_ascii() { CLASS } else { 0 }
    });
    ByteSet::excluding(&classes, CLASS)
}

fn custom_sets(data: &[u8]) -> Option<([ByteSet; 3], &[u8])> {
    let (&count, rest) = data.split_first()?;
    let (stops, rest) = rest.split_at_checked(1 + usize::from(count) % MAX_MEMBERS)?;
    let (&count, rest) = rest.split_first()?;
    let (marks, rest) = rest.split_at_checked(usize::from(count) % (MAX_MEMBERS + 1))?;
    let (bitmap, hay) = rest.split_first_chunk::<BITMAP_BYTES>()?;
    let bucket = |byte: u8| 1u8 << (byte >> 4);
    let mut stop_bytes = [0u8; MAX_MEMBERS];
    let mut stop_buckets = 0u8;
    for (out, &byte) in stop_bytes.iter_mut().zip(stops) {
        *out = byte & ASCII_MASK;
        stop_buckets |= bucket(*out);
    }
    let mut mark_bytes = [0u8; MAX_MEMBERS];
    let mut marked = 0;
    for byte in marks.iter().map(|&byte| byte & ASCII_MASK) {
        if bucket(byte) & stop_buckets == 0
            && let Some(out) = mark_bytes.get_mut(marked)
        {
            *out = byte;
            marked += 1;
        }
    }
    let classes: [u8; 256] = std::array::from_fn(|byte| {
        let bit = bitmap
            .get(byte / BITS)
            .is_some_and(|bits| bits >> (byte % BITS) & 1 == 1);
        if bit || byte >= usize::from(BELOW_LIMIT) {
            CLASS
        } else {
            0
        }
    });
    let stops = stop_bytes.get(..stops.len()).unwrap_or_default();
    let marks = mark_bytes.get(..marked).unwrap_or_default();
    Some((
        [
            ByteSet::new(stops),
            ByteSet::with_marks(stops, marks),
            ByteSet::excluding(&classes, CLASS),
        ],
        hay,
    ))
}

fn sets(set: &ByteSet, hay: &[u8]) {
    let len = hay.len();
    for end in [len.saturating_sub(1), len / 2, usize::MAX] {
        set_scans(set, hay, end);
    }
    variants(hay, SET_SHIFTS, |window| {
        set_scans(set, window, window.len())
    });
}

fn set_scans(set: &ByteSet, hay: &[u8], end: usize) {
    for kernel in scan::Kernel::available() {
        let mut from = 0;
        loop {
            let expected = scan::Kernel::SCALAR.first_in_set(set, hay, from, end);
            assert_eq!(
                kernel.first_in_set(set, hay, from, end),
                expected,
                "{} first_in_set {from}..{end} over {} bytes",
                kernel.name(),
                hay.len()
            );
            assert_eq!(
                kernel.stop_in_set(set, hay, from, end),
                scan::Kernel::SCALAR.stop_in_set(set, hay, from, end),
                "{} stop_in_set {from}..{end} over {} bytes",
                kernel.name(),
                hay.len()
            );
            match expected {
                Some(hit) => from = hit + 1,
                None => break,
            }
        }
        for from in [end, end.saturating_add(1), hay.len() + 1] {
            assert_eq!(
                kernel.first_in_set(set, hay, from, end),
                scan::Kernel::SCALAR.first_in_set(set, hay, from, end)
            );
            assert_eq!(
                kernel.stop_in_set(set, hay, from, end),
                scan::Kernel::SCALAR.stop_in_set(set, hay, from, end)
            );
        }
    }
}

fn words(hay: &[u8], low: u8, high: u8, limit: u8) {
    for chunk in hay.chunks(swar::WORD) {
        let mut bytes = [0u8; swar::WORD];
        bytes
            .iter_mut()
            .zip(chunk)
            .for_each(|(out, &byte)| *out = byte);
        let word = swar::load(chunk);
        assert_eq!(word, u64::from_le_bytes(bytes), "swar::load");
        let first = |mask: u64| (mask != 0).then(|| swar::first_byte(mask));
        assert_eq!(
            first(swar::equal_bytes(word, b':')),
            bytes.iter().position(|&byte| byte == b':')
        );
        assert_eq!(
            first(swar::zero_bytes(word)),
            bytes.iter().position(|&byte| byte == 0)
        );
        assert_eq!(
            first(swar::bytes_below(word, limit)),
            bytes.iter().position(|&byte| byte < limit),
            "bytes_below {limit}"
        );
        let range = swar::ascii_in_range(word, low, high);
        for (index, byte) in bytes.iter().enumerate() {
            assert_eq!(
                range >> (index * BITS + HIGH_BIT) & 1 == 1,
                (low..=high).contains(byte),
                "ascii_in_range {low}..={high} byte {byte}"
            );
        }
        assert_eq!(
            swar::ascii_lowercase(word).to_le_bytes(),
            bytes.map(|byte| byte.to_ascii_lowercase())
        );
    }
}
