/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use memchr::{memchr, memmem};

const MAX_NEEDLE: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Delimiter {
    pub(crate) frame: usize,
    pub(crate) line: usize,
    pub(crate) next: usize,
    pub(crate) is_close: bool,
    pub(crate) fallback: bool,
}

impl Delimiter {
    pub(crate) fn at_line(raw: &[u8], line: usize, frame: usize, boundary: &[u8]) -> Option<Self> {
        let after_dashes = raw.get(line..)?.strip_prefix(b"--")?;
        let is_close = match strip_boundary(after_dashes, boundary)? {
            [b'-', b'-', ..] => true,
            [] | [b'\n' | b'\r' | b' ' | b'\t', ..] => false,
            _ => return None,
        };
        Some(Delimiter {
            frame,
            line,
            next: line_after(raw, line + 2 + boundary.len()),
            is_close,
            fallback: false,
        })
    }

    pub(crate) fn mid_line(
        raw: &[u8],
        from: usize,
        limit: usize,
        frame: usize,
        boundary: &[u8],
    ) -> Option<Self> {
        let hay = raw.get(from..limit)?;
        let found = memmem::find_iter(hay, boundary)
            .find(|&pos| hay.get(..pos).is_some_and(|before| before.ends_with(b"--")))?;
        let line = from + found - 2;
        let after = line + 2 + boundary.len();
        Some(Delimiter {
            frame,
            line,
            next: line_after(raw, after),
            is_close: raw.get(after..).is_some_and(|rest| rest.starts_with(b"--")),
            fallback: true,
        })
    }

    pub(crate) fn content_end(&self, raw: &[u8], floor: usize) -> usize {
        let before = raw.get(floor..self.line).unwrap_or_default();
        let trimmed = before
            .strip_suffix(b"\n")
            .map_or(before, |line| line.strip_suffix(b"\r").unwrap_or(line));
        floor + trimmed.len()
    }
}

pub(crate) enum Dashed {
    At(usize),
    Absent,
    Unknown,
}

pub(crate) fn dashed_boundary(raw: &[u8], from: usize, boundary: &[u8]) -> Dashed {
    let mut buffer = [0u8; MAX_NEEDLE];
    let Some(needle) = buffer.get_mut(..boundary.len() + 2) else {
        return Dashed::Unknown;
    };
    let (dashes, name) = needle.split_at_mut(2);
    dashes.copy_from_slice(b"--");
    name.copy_from_slice(boundary);
    match raw.get(from..).and_then(|hay| memmem::find(hay, needle)) {
        Some(pos) => Dashed::At(from + pos),
        None => Dashed::Absent,
    }
}

fn strip_boundary<'a>(line: &'a [u8], boundary: &[u8]) -> Option<&'a [u8]> {
    if let (Some(head), Some(expected)) = (line.first_chunk::<8>(), boundary.first_chunk::<8>())
        && head != expected
    {
        return None;
    }
    line.strip_prefix(boundary)
}

fn line_after(raw: &[u8], from: usize) -> usize {
    raw.get(from..)
        .and_then(|tail| memchr(b'\n', tail))
        .map_or(raw.len(), |pos| from + pos + 1)
}

#[cfg(test)]
mod tests {
    use super::{Dashed, Delimiter, MAX_NEEDLE, dashed_boundary};

    #[test]
    fn delimiter_rule() {
        let raw = b"x\n--BND\r\n--BND--\n--BNDX\n--BND \t\n--BND";
        let at = |line| Delimiter::at_line(raw, line, 0, b"BND");
        let open = at(2).expect("open delimiter");
        assert!(!open.is_close && open.next == 9);
        assert!(at(9).is_some_and(|d| d.is_close));
        assert!(at(17).is_none());
        assert!(at(24).is_some_and(|d| !d.is_close));
        assert!(at(32).is_some_and(|d| !d.is_close && d.next == raw.len()));
        assert!(at(0).is_none());
    }

    #[test]
    fn mid_line_fallback() {
        let raw = b"text invalid--1-- tail\nnext";
        let found = Delimiter::mid_line(raw, 0, raw.len(), 3, b"1").expect("found");
        assert_eq!(found.line, 12);
        assert!(found.is_close && found.fallback);
        assert_eq!(found.next, 23);
        assert_eq!(found.content_end(raw, 0), 12);
        assert!(Delimiter::mid_line(raw, 0, 12, 3, b"1").is_none());
    }

    #[test]
    fn dashed_boundary_finds_the_leftmost_occurrence() {
        let found = |raw: &[u8], from, boundary: &[u8]| match dashed_boundary(raw, from, boundary) {
            Dashed::At(line) => Some(line),
            Dashed::Absent => None,
            Dashed::Unknown => Some(usize::MAX),
        };
        assert_eq!(
            found(b"\n--------------related\n", 0, b"----------"),
            Some(1)
        );
        assert_eq!(found(b"x--b\n--b--", 0, b"b"), Some(1));
        assert_eq!(found(b"x--b\n--b--", 2, b"b"), Some(5));
        assert_eq!(found(b"x--b", 3, b"b"), None);
        assert_eq!(found(b"boundary=b\n", 0, b"b"), None);
        let long = [b'a'; MAX_NEEDLE];
        assert_eq!(found(b"--aaaa", 0, &long), Some(usize::MAX));
    }

    #[test]
    fn content_end_trims_one_line_break() {
        let raw = b"body\r\n\r\n--b";
        let delimiter = Delimiter::at_line(raw, 8, 0, b"b").expect("valid");
        assert_eq!(delimiter.content_end(raw, 0), 6);
        assert_eq!(delimiter.content_end(raw, 8), 8);
    }
}
