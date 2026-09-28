/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{borrow::Cow, char::REPLACEMENT_CHARACTER, ops::RangeInclusive};

use memchr::memchr;

use super::ascii::split_ascii;

const SHIFT: u8 = b'+';
const ABSORBED: u8 = b'-';
const LEAD_SURROGATES: RangeInclusive<u16> = 0xd800..=0xdbff;
const TRAIL_SURROGATES: RangeInclusive<u16> = 0xdc00..=0xdfff;

trait Report {
    fn malformed(&mut self);
}

impl Report for () {
    fn malformed(&mut self) {}
}

impl Report for bool {
    fn malformed(&mut self) {
        *self = true;
    }
}

pub(super) fn decode(bytes: &[u8]) -> Cow<'_, str> {
    decode_to_cow(bytes, &mut ())
}

pub(super) fn decode_checked(bytes: &[u8]) -> (Cow<'_, str>, bool) {
    let mut malformed = false;
    let text = decode_to_cow(bytes, &mut malformed);
    (text, malformed)
}

fn decode_to_cow<'x>(bytes: &'x [u8], report: &mut impl Report) -> Cow<'x, str> {
    if let (ascii, []) = split_ascii(bytes)
        && memchr(SHIFT, bytes).is_none()
    {
        return Cow::Borrowed(ascii);
    }
    let mut out = String::new();
    decode_reporting(bytes, &mut out, report);
    Cow::Owned(out)
}

pub(super) fn decode_append(bytes: &[u8], out: &mut String) {
    decode_reporting(bytes, out, &mut ());
}

fn decode_reporting(bytes: &[u8], out: &mut String, report: &mut impl Report) {
    out.reserve(bytes.len());
    let mut run: Option<Run> = None;
    for &byte in bytes {
        match run.as_mut() {
            Some(state) => match sextet(byte) {
                Some(value) => state.push_sextet(value, out, report),
                None => {
                    state.finish(byte, out, report);
                    run = None;
                }
            },
            None if byte == SHIFT => run = Some(Run::new(out.len())),
            None => push_direct(byte, out, report),
        }
    }
    if let Some(state) = run {
        report.malformed();
        out.truncate(state.start);
    }
}

pub(super) fn complete_prefix(bytes: &[u8]) -> &[u8] {
    let mut open_run = None;
    for (pos, &byte) in bytes.iter().enumerate() {
        if open_run.is_some() {
            if sextet(byte).is_none() {
                open_run = None;
            }
        } else if byte == SHIFT {
            open_run = Some(pos);
        }
    }
    open_run.map_or(bytes, |start| bytes.get(..start).unwrap_or_default())
}

struct Run {
    start: usize,
    bits: u32,
    count: u8,
    pending: Option<u8>,
    lead: Option<u16>,
    has_units: bool,
}

impl Run {
    fn new(start: usize) -> Self {
        Run {
            start,
            bits: 0,
            count: 0,
            pending: None,
            lead: None,
            has_units: false,
        }
    }

    fn push_sextet(&mut self, value: u8, out: &mut String, report: &mut impl Report) {
        self.bits = (self.bits << 6) | u32::from(value);
        self.count += 1;
        if self.count == 4 {
            let [_, first, second, third] = self.bits.to_be_bytes();
            self.push_byte(first, out, report);
            self.push_byte(second, out, report);
            self.push_byte(third, out, report);
            self.bits = 0;
            self.count = 0;
        }
    }

    fn finish(&mut self, terminator: u8, out: &mut String, report: &mut impl Report) {
        match self.count {
            1 => self.push_byte((self.bits << 2) as u8, out, report),
            2 => self.push_byte((self.bits >> 4) as u8, out, report),
            3 => {
                self.push_byte((self.bits >> 10) as u8, out, report);
                self.push_byte((self.bits >> 2) as u8, out, report);
            }
            _ => (),
        }
        if self.has_units {
            if self.lead.is_some() {
                report.malformed();
                out.push(REPLACEMENT_CHARACTER);
            }
        } else if self.count > 0 || self.pending.is_some() {
            report.malformed();
            out.push(REPLACEMENT_CHARACTER);
        } else {
            out.push(char::from(SHIFT));
        }
        if terminator != ABSORBED {
            push_direct(terminator, out, report);
        }
    }

    fn push_byte(&mut self, byte: u8, out: &mut String, report: &mut impl Report) {
        match self.pending.take() {
            Some(high) => self.push_unit(u16::from_be_bytes([high, byte]), out, report),
            None => self.pending = Some(byte),
        }
    }

    fn push_unit(&mut self, unit: u16, out: &mut String, report: &mut impl Report) {
        self.has_units = true;
        match self.lead.take() {
            Some(lead) if TRAIL_SURROGATES.contains(&unit) => {
                push_units(&[lead, unit], out, report)
            }
            Some(_) => {
                report.malformed();
                out.push(REPLACEMENT_CHARACTER);
                self.start_unit(unit, out, report);
            }
            None => self.start_unit(unit, out, report),
        }
    }

    fn start_unit(&mut self, unit: u16, out: &mut String, report: &mut impl Report) {
        if LEAD_SURROGATES.contains(&unit) {
            self.lead = Some(unit);
        } else {
            push_units(&[unit], out, report);
        }
    }
}

fn push_direct(byte: u8, out: &mut String, report: &mut impl Report) {
    if !byte.is_ascii() {
        report.malformed();
    }
    out.push(char::from(byte));
}

fn push_units(units: &[u16], out: &mut String, report: &mut impl Report) {
    out.extend(char::decode_utf16(units.iter().copied()).map(|result| {
        result.unwrap_or_else(|_| {
            report.malformed();
            REPLACEMENT_CHARACTER
        })
    }));
}

fn sextet(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
