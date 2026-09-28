/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#[cfg(feature = "full_encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};

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
            .get(..UNIT + Endian::Big.complete_prefix(rest).len())
            .unwrap_or_default(),
        [0xff, 0xfe, rest @ ..] => bytes
            .get(..UNIT + Endian::Little.complete_prefix(rest).len())
            .unwrap_or_default(),
        [_, _, ..] => Endian::Little.complete_prefix(bytes),
        _ => &[],
    }
}

pub(super) fn decode_bom(bytes: &[u8]) -> String {
    let mut out = String::new();
    decode_bom_append(bytes, &mut out);
    out
}

pub(super) fn decode_bom_append(bytes: &[u8], out: &mut String) {
    match bytes {
        [0xfe, 0xff, rest @ ..] => Endian::Big.decode_append(rest, out),
        [0xff, 0xfe, rest @ ..] => Endian::Little.decode_append(rest, out),
        _ => Endian::Little.decode_append(bytes, out),
    }
}

pub(super) fn decode_bom_checked(bytes: &[u8]) -> (String, bool) {
    match bytes {
        [0xfe, 0xff, rest @ ..] => Endian::Big.decode_checked(rest),
        [0xff, 0xfe, rest @ ..] => Endian::Little.decode_checked(rest),
        _ => Endian::Little.decode_checked(bytes),
    }
}

impl Endian {
    fn unit(self, bytes: [u8; UNIT]) -> u16 {
        match self {
            Endian::Little => u16::from_le_bytes(bytes),
            Endian::Big => u16::from_be_bytes(bytes),
        }
    }

    pub(super) fn complete_prefix(self, bytes: &[u8]) -> &[u8] {
        let units = bytes.as_chunks::<UNIT>().0;
        let complete = match units.last() {
            Some(&unit) if LEAD_SURROGATES.contains(&self.unit(unit)) => units.len() - 1,
            _ => units.len(),
        };
        bytes.get(..complete * UNIT).unwrap_or_default()
    }

    pub(super) fn decode(self, bytes: &[u8]) -> String {
        let mut out = String::new();
        self.decode_append(bytes, &mut out);
        out
    }

    #[cfg(feature = "full_encoding")]
    pub(super) fn decode_checked(self, bytes: &[u8]) -> (String, bool) {
        let (text, replaced) = self.encoding().decode_without_bom_handling(bytes);
        (text.into_owned(), replaced)
    }

    #[cfg(not(feature = "full_encoding"))]
    pub(super) fn decode_checked(self, bytes: &[u8]) -> (String, bool) {
        use std::char::{REPLACEMENT_CHARACTER, decode_utf16};

        let (units, odd) = bytes.as_chunks::<UNIT>();
        let mut malformed = !odd.is_empty();
        let mut out = String::with_capacity(units.len());
        out.extend(
            decode_utf16(units.iter().map(|&unit| self.unit(unit))).map(|result| {
                result.unwrap_or_else(|_| {
                    malformed = true;
                    REPLACEMENT_CHARACTER
                })
            }),
        );
        if self.replaces_odd_byte(units, odd) {
            out.push(REPLACEMENT_CHARACTER);
        }
        (out, malformed)
    }

    #[cfg(not(feature = "full_encoding"))]
    fn replaces_odd_byte(self, units: &[[u8; UNIT]], odd: &[u8]) -> bool {
        !odd.is_empty()
            && !units
                .last()
                .is_some_and(|&unit| LEAD_SURROGATES.contains(&self.unit(unit)))
    }

    #[cfg(feature = "full_encoding")]
    pub(super) fn decode_append(self, bytes: &[u8], out: &mut String) {
        super::multi_byte::decode_with(
            self.encoding().new_decoder_without_bom_handling(),
            bytes,
            out,
        );
    }

    #[cfg(feature = "full_encoding")]
    fn encoding(self) -> &'static Encoding {
        match self {
            Endian::Little => UTF_16LE,
            Endian::Big => UTF_16BE,
        }
    }

    #[cfg(not(feature = "full_encoding"))]
    pub(super) fn decode_append(self, bytes: &[u8], out: &mut String) {
        use std::char::{REPLACEMENT_CHARACTER, decode_utf16};

        let (units, odd) = bytes.as_chunks::<UNIT>();
        out.reserve(units.len());
        out.extend(
            decode_utf16(units.iter().map(|&unit| self.unit(unit)))
                .map(|result| result.unwrap_or(REPLACEMENT_CHARACTER)),
        );
        if self.replaces_odd_byte(units, odd) {
            out.push(REPLACEMENT_CHARACTER);
        }
    }
}
