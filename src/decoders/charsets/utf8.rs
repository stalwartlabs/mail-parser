/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{borrow::Cow, char::REPLACEMENT_CHARACTER};

use crate::store::validated_string;
use simdutf8::{basic, compat};

const MAX_INCOMPLETE: usize = 3;
pub(super) const SIMD_MIN: usize = 4096;

pub(super) fn decode(bytes: &[u8]) -> Cow<'_, str> {
    match compat::from_utf8(bytes) {
        Ok(text) => Cow::Borrowed(text),
        Err(error) => {
            let mut out = String::with_capacity(bytes.len());
            push_lossy(bytes, error.valid_up_to(), &mut out);
            Cow::Owned(out)
        }
    }
}

pub(super) fn decode_append(bytes: &[u8], out: &mut String) {
    match compat::from_utf8(bytes) {
        Ok(text) => out.push_str(text),
        Err(error) => {
            out.reserve(bytes.len());
            push_lossy(bytes, error.valid_up_to(), out);
        }
    }
}

pub(super) fn decode_checked(bytes: &[u8]) -> (Cow<'_, str>, bool) {
    let text = decode(bytes);
    let malformed = matches!(text, Cow::Owned(_));
    (text, malformed)
}

pub(super) fn decode_owned(bytes: Vec<u8>) -> String {
    decode_owned_checked(bytes).0
}

pub(super) fn decode_owned_checked(bytes: Vec<u8>) -> (String, bool) {
    if bytes.len() >= SIMD_MIN {
        return match validated_string(bytes) {
            Ok(text) => (text, false),
            Err(bytes) => (decode(&bytes).into_owned(), true),
        };
    }
    match String::from_utf8(bytes) {
        Ok(text) => (text, false),
        Err(err) => (String::from_utf8_lossy(err.as_bytes()).into_owned(), true),
    }
}

fn push_lossy(bytes: &[u8], valid_up_to: usize, out: &mut String) {
    let (valid, rest) = bytes.split_at(valid_up_to);
    out.push_str(basic::from_utf8(valid).unwrap_or_default());
    for chunk in rest.utf8_chunks() {
        out.push_str(chunk.valid());
        if !chunk.invalid().is_empty() {
            out.push(REPLACEMENT_CHARACTER);
        }
    }
}

pub(super) fn complete_prefix(bytes: &[u8]) -> &[u8] {
    for back in 1..=MAX_INCOMPLETE {
        let Some(start) = bytes.len().checked_sub(back) else {
            break;
        };
        let (complete, tail) = bytes.split_at(start);
        match tail.first() {
            Some(byte) if is_continuation(*byte) => continue,
            Some(_) => {
                return match std::str::from_utf8(tail) {
                    Err(error) if error.valid_up_to() == 0 && error.error_len().is_none() => {
                        complete
                    }
                    _ => bytes,
                };
            }
            None => break,
        }
    }
    bytes
}

fn is_continuation(byte: u8) -> bool {
    byte & 0xc0 == 0x80
}
