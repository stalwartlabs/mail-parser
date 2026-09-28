/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::borrow::Cow;

use simdutf8::basic::from_utf8;

use super::ascii::{as_ascii_str, ascii_len, non_ascii_count, split_ascii};

const ASCII_LEN: usize = 128;
const EXTRA_BYTES_PER_CHAR: usize = 2;
const SPARSE_RATIO: usize = 64;
const ENTRY_LEN: usize = 4;
const LENGTH_BYTE: usize = 3;
const MAX_CHAR_LEN: usize = 3;
const TINY_CHUNK: usize = 32;
const TINY_MASK: usize = 127;
const SMALL_CHUNK: usize = 128;
const SMALL_MASK: usize = 511;
const LARGE_CHUNK: usize = 512;
const LARGE_MASK: usize = 2047;
const SIMD_VALIDATION_LEN: usize = 64;

pub(super) struct Table {
    chars: [char; 256],
    encoded: [[u8; ENTRY_LEN]; 256],
}

impl Table {
    pub(super) const fn new(upper: [char; ASCII_LEN]) -> Table {
        let mut chars = ['\0'; 256];
        let mut encoded = [[0u8; ENTRY_LEN]; 256];
        let mut index = 0;
        while index < chars.len() {
            let ch = if index < ASCII_LEN {
                index as u8 as char
            } else {
                upper[index - ASCII_LEN]
            };
            let code = ch as u32;
            chars[index] = ch;
            encoded[index] = if code < 0x80 {
                [code as u8, 0, 0, 1]
            } else if code < 0x800 {
                [0xc0 | (code >> 6) as u8, 0x80 | (code & 0x3f) as u8, 0, 2]
            } else {
                assert!(code < 0x10000);
                [
                    0xe0 | (code >> 12) as u8,
                    0x80 | ((code >> 6) & 0x3f) as u8,
                    0x80 | (code & 0x3f) as u8,
                    3,
                ]
            };
            index += 1;
        }
        Table { chars, encoded }
    }
}

pub(super) fn decode<'x>(table: &Table, bytes: &'x [u8]) -> Cow<'x, str> {
    if bytes.len() <= SMALL_CHUNK {
        if ascii_len(bytes) == bytes.len() {
            return Cow::Borrowed(as_ascii_str(bytes));
        }
        let mut out = String::with_capacity(bytes.len() * MAX_CHAR_LEN);
        push_short(table, bytes, &mut out);
        return Cow::Owned(out);
    }
    match split_ascii(bytes) {
        (ascii, []) => Cow::Borrowed(ascii),
        (ascii, rest) => {
            let non_ascii = non_ascii_count(rest);
            let mut out =
                String::with_capacity(ascii.len() + rest.len() + non_ascii * EXTRA_BYTES_PER_CHAR);
            out.push_str(ascii);
            push_decoded(table, rest, non_ascii, &mut out);
            Cow::Owned(out)
        }
    }
}

pub(super) fn decode_append(table: &Table, bytes: &[u8], out: &mut String) {
    if bytes.len() <= SMALL_CHUNK {
        out.reserve(bytes.len() * MAX_CHAR_LEN);
        push_short(table, bytes, out);
        return;
    }
    let (ascii, rest) = split_ascii(bytes);
    let non_ascii = non_ascii_count(rest);
    out.reserve(ascii.len() + rest.len() + non_ascii * EXTRA_BYTES_PER_CHAR);
    out.push_str(ascii);
    push_decoded(table, rest, non_ascii, out);
}

fn push_decoded(table: &Table, bytes: &[u8], non_ascii: usize, out: &mut String) {
    if non_ascii * SPARSE_RATIO < bytes.len() {
        push_runs(table, bytes, out);
    } else if bytes.len() <= SMALL_CHUNK {
        push_short(table, bytes, out);
    } else {
        push_chunks::<LARGE_CHUNK, LARGE_MASK, { LARGE_MASK + 1 + ENTRY_LEN }>(table, bytes, out);
    }
}

fn push_short(table: &Table, bytes: &[u8], out: &mut String) {
    if bytes.len() <= TINY_CHUNK {
        push_chunks::<TINY_CHUNK, TINY_MASK, { TINY_MASK + 1 + ENTRY_LEN }>(table, bytes, out);
    } else {
        push_chunks::<SMALL_CHUNK, SMALL_MASK, { SMALL_MASK + 1 + ENTRY_LEN }>(table, bytes, out);
    }
}

fn push_runs(table: &Table, mut bytes: &[u8], out: &mut String) {
    while let Some((&byte, rest)) = bytes.split_first() {
        out.push(table.chars[usize::from(byte)]);
        let (ascii, rest) = split_ascii(rest);
        out.push_str(ascii);
        bytes = rest;
    }
}

fn push_chars(table: &Table, bytes: &[u8], out: &mut String) {
    for &byte in bytes {
        out.push(table.chars[usize::from(byte)]);
    }
}

fn push_chunks<const CHUNK: usize, const MASK: usize, const BUFFER: usize>(
    table: &Table,
    bytes: &[u8],
    out: &mut String,
) {
    const {
        assert!(CHUNK * MAX_CHAR_LEN <= MASK && (MASK + 1).is_power_of_two());
        assert!(BUFFER == MASK + 1 + ENTRY_LEN && BUFFER >= SIMD_VALIDATION_LEN);
    };
    let mut buffer = [0u8; BUFFER];
    let single_chunk = bytes.len() <= CHUNK;
    for chunk in bytes.chunks(CHUNK) {
        let mut len = 0;
        for &byte in chunk {
            let entry = table.encoded[usize::from(byte)];
            let start = len & MASK;
            if let Some(slot) = buffer.get_mut(start..start + ENTRY_LEN) {
                slot.copy_from_slice(&entry);
            }
            len = start + usize::from(entry[LENGTH_BYTE]);
        }
        let validated = if single_chunk {
            len.max(SIMD_VALIDATION_LEN)
        } else {
            len
        };
        match buffer
            .get(..validated)
            .and_then(|bytes| from_utf8(bytes).ok())
            .and_then(|text| text.get(..len))
        {
            Some(text) => out.push_str(text),
            None => push_chars(table, chunk, out),
        }
    }
}
