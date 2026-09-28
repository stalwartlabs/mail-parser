/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Endian {
    Little,
    Big,
}

const UNIT: usize = 2;
const LEAD_SURROGATES: std::ops::RangeInclusive<u16> = 0xd800..=0xdbff;

pub(super) fn complete_prefix_bom(bytes: &[u8]) -> &[u8] {
    match bytes {
        [0xfe, 0xff, rest @ ..] => bytes
            .get(..UNIT + complete_prefix(rest, Endian::Big).len())
            .unwrap_or_default(),
        [0xff, 0xfe, rest @ ..] => bytes
            .get(..UNIT + complete_prefix(rest, Endian::Little).len())
            .unwrap_or_default(),
        [_, _, ..] => complete_prefix(bytes, Endian::Little),
        _ => &[],
    }
}

pub(super) fn complete_prefix(bytes: &[u8], endian: Endian) -> &[u8] {
    let units = bytes.as_chunks::<UNIT>().0;
    let complete = match units.last() {
        Some(&unit) if LEAD_SURROGATES.contains(&endian.unit(unit)) => units.len() - 1,
        _ => units.len(),
    };
    bytes.get(..complete * UNIT).unwrap_or_default()
}

impl Endian {
    fn unit(self, bytes: [u8; UNIT]) -> u16 {
        match self {
            Endian::Little => u16::from_le_bytes(bytes),
            Endian::Big => u16::from_be_bytes(bytes),
        }
    }
}

pub(super) fn decode_bom(bytes: &[u8]) -> String {
    let mut out = String::new();
    decode_bom_append(bytes, &mut out);
    out
}

pub(super) fn decode_bom_append(bytes: &[u8], out: &mut String) {
    match bytes {
        [0xfe, 0xff, rest @ ..] => decode_append(rest, Endian::Big, out),
        [0xff, 0xfe, rest @ ..] => decode_append(rest, Endian::Little, out),
        _ => decode_append(bytes, Endian::Little, out),
    }
}

pub(super) fn decode(bytes: &[u8], endian: Endian) -> String {
    let mut out = String::new();
    decode_append(bytes, endian, &mut out);
    out
}

pub(super) fn decode_bom_checked(bytes: &[u8]) -> (String, bool) {
    match bytes {
        [0xfe, 0xff, rest @ ..] => decode_checked(rest, Endian::Big),
        [0xff, 0xfe, rest @ ..] => decode_checked(rest, Endian::Little),
        _ => decode_checked(bytes, Endian::Little),
    }
}

#[cfg(feature = "full_encoding")]
pub(super) fn decode_checked(bytes: &[u8], endian: Endian) -> (String, bool) {
    use encoding_rs::{UTF_16BE, UTF_16LE};

    let encoding = match endian {
        Endian::Little => UTF_16LE,
        Endian::Big => UTF_16BE,
    };
    let (text, replaced) = encoding.decode_without_bom_handling(bytes);
    (text.into_owned(), replaced)
}

#[cfg(not(feature = "full_encoding"))]
pub(super) fn decode_checked(bytes: &[u8], endian: Endian) -> (String, bool) {
    use std::char::{REPLACEMENT_CHARACTER, decode_utf16};

    let (units, odd) = bytes.as_chunks::<UNIT>();
    let mut malformed = !odd.is_empty();
    let mut out = String::with_capacity(units.len());
    out.extend(
        decode_utf16(units.iter().map(|&unit| endian.unit(unit))).map(|result| {
            result.unwrap_or_else(|_| {
                malformed = true;
                REPLACEMENT_CHARACTER
            })
        }),
    );
    if replaces_odd_byte(units, odd, endian) {
        out.push(REPLACEMENT_CHARACTER);
    }
    (out, malformed)
}

#[cfg(not(feature = "full_encoding"))]
fn replaces_odd_byte(units: &[[u8; UNIT]], odd: &[u8], endian: Endian) -> bool {
    !odd.is_empty()
        && !units
            .last()
            .is_some_and(|&unit| LEAD_SURROGATES.contains(&endian.unit(unit)))
}

#[cfg(feature = "full_encoding")]
pub(super) fn decode_append(bytes: &[u8], endian: Endian, out: &mut String) {
    use encoding_rs::{UTF_16BE, UTF_16LE};

    let encoding = match endian {
        Endian::Little => UTF_16LE,
        Endian::Big => UTF_16BE,
    };
    super::multi_byte::decode_with(encoding.new_decoder_without_bom_handling(), bytes, out);
}

#[cfg(not(feature = "full_encoding"))]
pub(super) fn decode_append(bytes: &[u8], endian: Endian, out: &mut String) {
    use std::char::{REPLACEMENT_CHARACTER, decode_utf16};

    let (units, odd) = bytes.as_chunks::<UNIT>();
    out.reserve(units.len());
    out.extend(
        decode_utf16(units.iter().map(|&unit| endian.unit(unit)))
            .map(|result| result.unwrap_or(REPLACEMENT_CHARACTER)),
    );
    if replaces_odd_byte(units, odd, endian) {
        out.push(REPLACEMENT_CHARACTER);
    }
}
