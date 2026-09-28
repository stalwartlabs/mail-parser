/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{ByteSet, Kernel, set};
use std::path::PathBuf;

const ALPHABET: &[u8] = b"\n\n\n\r\r---- \ta";
const ITERATIONS: usize = 3_000;
const MAX_LEN: usize = 300;

pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub(crate) fn below(&mut self, limit: usize) -> usize {
        (self.next() % limit.max(1) as u64) as usize
    }
}

fn agree(kernel: Kernel, hay: &[u8]) {
    for from in 0..=hay.len() + 1 {
        assert_eq!(
            kernel.dash_line(hay, from),
            Kernel::SCALAR.dash_line(hay, from),
            "{} dash_line from {from} in {hay:?}",
            kernel.name()
        );
        assert_eq!(
            kernel.field_end(hay, from),
            Kernel::SCALAR.field_end(hay, from),
            "{} field_end from {from} in {hay:?}",
            kernel.name()
        );
    }
}

fn all_hits(hay: &[u8], find: impl Fn(&[u8], usize) -> Option<usize>) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut from = 0;
    while let Some(hit) = find(hay, from) {
        hits.push(hit);
        from = hit + 1;
    }
    hits
}

fn agree_on_sample(kernel: Kernel, hay: &[u8]) {
    assert_eq!(
        all_hits(hay, |h, f| kernel.dash_line(h, f)),
        all_hits(hay, |h, f| Kernel::SCALAR.dash_line(h, f)),
        "{} dash_line",
        kernel.name()
    );
    assert_eq!(
        all_hits(hay, |h, f| kernel.field_end(h, f)),
        all_hits(hay, |h, f| Kernel::SCALAR.field_end(h, f)),
        "{} field_end",
        kernel.name()
    );
}

pub(crate) fn fixture_files() -> Vec<Vec<u8>> {
    let mut files = Vec::new();
    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/eml")];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "eml")
                && let Ok(bytes) = std::fs::read(&path)
            {
                files.push(bytes);
            }
        }
    }
    files
}

pub(crate) fn with_crlf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + bytes.len() / 16);
    let mut last = 0u8;
    for &byte in bytes {
        if byte == b'\n' && last != b'\r' {
            out.push(b'\r');
        }
        out.push(byte);
        last = byte;
    }
    out
}

pub(crate) fn with_lf(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .copied()
        .filter(|&byte| byte != b'\r')
        .collect()
}

#[test]
fn kernels_match_scalar_on_random_input() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..ITERATIONS {
        let len = rng.below(MAX_LEN);
        let hay: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect();
        for kernel in Kernel::available() {
            agree(kernel, &hay);
        }
    }
}

#[test]
fn kernels_match_scalar_on_adversarial_input() {
    let patterns: [&[u8]; 8] = [
        b"\n",
        b"\n-",
        b"\n--",
        b"\n \n\t\n",
        b"--\n--\n",
        b"\r\n\r\n--",
        b"\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n",
        b"-----------------------------------",
    ];
    for pattern in patterns {
        for pad in 0..70 {
            for tail in 0..3 {
                let mut hay = vec![b'x'; pad];
                hay.extend_from_slice(pattern);
                hay.extend(std::iter::repeat_n(b'y', tail));
                for kernel in Kernel::available() {
                    agree(kernel, &hay);
                }
            }
        }
    }
    let mut dense = Vec::new();
    for index in 0..512usize {
        dense.push(match index % 7 {
            0 | 3 => b'\n',
            1 | 4 => b'-',
            2 => b' ',
            5 => b'\t',
            _ => b'-',
        });
    }
    for kernel in Kernel::available() {
        agree(kernel, &dense);
    }
}

#[test]
fn kernels_match_scalar_on_fixtures() {
    for file in fixture_files() {
        for hay in [with_lf(&file), with_crlf(&file)] {
            for kernel in Kernel::available() {
                agree_on_sample(kernel, &hay);
            }
        }
    }
}

#[test]
fn best_kernel_is_available() {
    assert!(Kernel::available().any(|kernel| kernel == Kernel::best()));
}

const SETS: [ByteSet; 7] = [
    ByteSet::with_marks(b"\n\r,;<\"(:=", b"@"),
    ByteSet::new(b"\n\r,;<\"(:="),
    ByteSet::new(b"\n\r\"\\="),
    ByteSet::new(b"\n\r>\\(="),
    ByteSet::new(b"@"),
    ByteSet::new(b"\x00\x7f"),
    ByteSet::new(b" !\"#$%&'()*+,-.0"),
];

fn agree_on_sets(hay: &[u8]) {
    for set in &SETS {
        for from in 0..=hay.len() + 1 {
            for end in [
                from,
                from + 1,
                from + 7,
                from + 16,
                from + 17,
                from + 40,
                usize::MAX,
            ] {
                let expected = set::first_in_set(set, hay, from, end);
                let window = hay.get(from..end.min(hay.len())).unwrap_or_default();
                let naive = hay
                    .get(from..end.min(hay.len()))
                    .and_then(|bytes| bytes.iter().position(|byte| set.contains(*byte)));
                assert_eq!(expected, naive.map(|pos| pos + from));
                let marked = window
                    .get(..naive.unwrap_or(window.len()))
                    .unwrap_or_default()
                    .iter()
                    .any(|byte| set.marks(*byte));
                let stop = set::stop_in_set(set, hay, from, end);
                assert_eq!((stop.at, stop.marked), (expected, marked));
                for kernel in Kernel::available() {
                    assert_eq!(
                        kernel.first_in_set(set, hay, from, end),
                        expected,
                        "{} first_in_set from {from} end {end} in {hay:?}",
                        kernel.name()
                    );
                    assert_eq!(
                        kernel.stop_in_set(set, hay, from, end),
                        stop,
                        "{} stop_in_set from {from} end {end} in {hay:?}",
                        kernel.name()
                    );
                }
            }
        }
    }
}

#[test]
fn byte_sets_are_exact() {
    for (set, members) in SETS.iter().zip([
        &b"\n\r,;<\"(:="[..],
        b"\n\r,;<\"(:=",
        b"\n\r\"\\=",
        b"\n\r>\\(=",
        b"@",
        b"\x00\x7f",
        b" !\"#$%&'()*+,-.0",
    ]) {
        for byte in 0..=u8::MAX {
            assert_eq!(set.contains(byte), members.contains(&byte), "{byte:#x}");
        }
    }
    let marked = SETS.first().copied().unwrap_or(ByteSet::new(b"@"));
    for byte in 0..=u8::MAX {
        assert_eq!(marked.marks(byte), byte == b'@', "{byte:#x}");
    }
}

const ALNUM_CLASSES: [u8; 256] = {
    let mut table = [0; 256];
    let mut byte = 0;
    while byte < 256 {
        if (byte as u8).is_ascii_alphanumeric() || byte >= 0x80 {
            table[byte] = 1;
        }
        byte += 1;
    }
    table
};

static NOT_ALNUM: ByteSet = ByteSet::excluding(&ALNUM_CLASSES, 1);

#[test]
fn excluding_sets_are_exact() {
    for byte in 0..=u8::MAX {
        assert_eq!(
            NOT_ALNUM.contains(byte),
            byte < 0x80 && !byte.is_ascii_alphanumeric(),
            "{byte:#x}"
        );
    }
    let hay: Vec<u8> = (0..=u8::MAX).rev().chain(0..=u8::MAX).collect();
    for from in 0..hay.len() {
        for kernel in Kernel::available() {
            assert_eq!(
                kernel.first_in_set(&NOT_ALNUM, &hay, from, hay.len()),
                set::first_in_set(&NOT_ALNUM, &hay, from, hay.len()),
                "{} from {from}",
                kernel.name()
            );
        }
    }
}

#[test]
fn set_kernels_match_scalar() {
    let alphabet: &[u8] = b"aaaaaaaaaa@<>\"\\,;:=()\n\r \t\x00\x7f\x80\xc3\xff0/";
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..400 {
        let len = rng.below(90);
        let hay: Vec<u8> = (0..len)
            .map(|_| {
                alphabet
                    .get(rng.below(alphabet.len()))
                    .copied()
                    .unwrap_or(b'a')
            })
            .collect();
        agree_on_sets(&hay);
    }
    for len in 0..40 {
        for hit in 0..len {
            let mut hay = vec![b'a'; len];
            if let Some(byte) = hay.get_mut(hit) {
                *byte = b'@';
            }
            agree_on_sets(&hay);
        }
    }
}
