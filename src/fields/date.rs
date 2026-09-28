/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::FieldCtx;
use crate::{DateTime, store::Value};
use std::{ops::Range, slice::Iter};

const SLOTS: usize = 7;
const MONTH_SLOT: usize = 1;
const ZONE_SLOT: usize = 6;
const WIDTHS: [u32; SLOTS] = [2, 2, 4, 2, 2, 2, 4];
const POW10: [u32; 5] = [1, 10, 100, 1000, 10000];
const FIXED_TAIL: usize = 23;
const LANES: u64 = 0x0101_0101_0101_0101;
const HIGH_BITS: u64 = 0x8080_8080_8080_8080;
const DIGIT_CEILING: u64 = 0x7676_7676_7676_7676;
const MONTH_YEAR: Layout = Layout::new(b"??? DDDD");
const TIME: Layout = Layout::new(b" DD:DD:D");
const SECOND_ZONE: Layout = Layout::new(b"DD ?DDDD");

static LETTER_HASH: [u8; 26] = [
    0, 14, 4, 31, 10, 31, 14, 31, 31, 31, 31, 4, 31, 10, 15, 15, 31, 5, 31, 0, 5, 15, 31, 31, 0, 31,
];

static MONTH_MAP: [u8; 31] = [
    5, 0, 0, 0, 10, 3, 0, 0, 0, 7, 1, 0, 0, 0, 12, 6, 0, 0, 0, 8, 4, 0, 0, 0, 2, 9, 0, 0, 0, 0, 11,
];

impl FieldCtx<'_> {
    pub(crate) fn parse_date(&mut self, value: Range<usize>) -> Value {
        parse_bytes(self.bytes(value)).map_or(Value::Empty, Value::DateTime)
    }
}

pub(crate) fn parse_bytes(bytes: &[u8]) -> Option<DateTime> {
    canonical(bytes).or_else(|| Scanner::scan(bytes))
}

fn canonical(bytes: &[u8]) -> Option<DateTime> {
    let rest = skip_day_name(skip_space(bytes))?;
    let (day, rest) = match rest {
        [d1 @ b'0'..=b'9', d2 @ b'0'..=b'9', rest @ ..] => (two_digits(*d1, *d2), rest),
        [d1 @ b'0'..=b'9', rest @ ..] => (u32::from(d1 - b'0'), rest),
        _ => return None,
    };
    let rest = separator(rest)?;
    if let Some(date) = fixed_tail(day, rest) {
        return Some(date);
    }
    let (month, rest) = month_name(rest)?;
    let (year, rest) = match separator(rest)? {
        [
            y1 @ b'0'..=b'9',
            y2 @ b'0'..=b'9',
            y3 @ b'0'..=b'9',
            y4 @ b'0'..=b'9',
            rest @ ..,
        ] => (two_digits(*y1, *y2) * 100 + two_digits(*y3, *y4), rest),
        rest => number(rest, 4)?,
    };
    let (hour, minute, second, rest) = match separator(rest)? {
        [
            h1 @ b'0'..=b'9',
            h2 @ b'0'..=b'9',
            b':',
            m1 @ b'0'..=b'9',
            m2 @ b'0'..=b'9',
            b':',
            s1 @ b'0'..=b'9',
            s2 @ b'0'..=b'9',
            rest @ ..,
        ] => (
            two_digits(*h1, *h2),
            two_digits(*m1, *m2),
            Some(two_digits(*s1, *s2)),
            rest,
        ),
        rest => time(rest)?,
    };
    let (tz_before_gmt, zone) = match separator(rest)? {
        [
            sign @ (b'+' | b'-'),
            h1 @ b'0'..=b'9',
            h2 @ b'0'..=b'9',
            m1 @ b'0'..=b'9',
            m2 @ b'0'..=b'9',
            rest @ ..,
        ] => {
            numeric_zone_end(rest)?;
            (
                *sign == b'-',
                two_digits(*h1, *h2) * 100 + two_digits(*m1, *m2),
            )
        }
        [letter, rest @ ..] if second.is_some() && letter.is_ascii_alphabetic() => {
            let (key, rest) = obs_key(*letter, rest);
            inert(rest)?;
            let zone = obs_zone(key);
            (zone < 0, 100 * zone.unsigned_abs())
        }
        _ => return None,
    };
    Some(date_time(
        [day, 0, year, hour, minute, second.unwrap_or(0), zone],
        month,
        tz_before_gmt,
    ))
}

fn date_time(parts: [u32; SLOTS], month: u8, tz_before_gmt: bool) -> DateTime {
    let [day, _, year, hour, minute, second, zone] = parts;
    DateTime {
        year: match year {
            0..=49 => year + 2000,
            50..=99 => year + 1900,
            _ => year,
        } as u16,
        month,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
        tz_before_gmt,
        tz_hour: ((zone / 100) % 24) as u8,
        tz_minute: ((zone % 100) % 60) as u8,
    }
}

fn fixed_tail(day: u32, bytes: &[u8]) -> Option<DateTime> {
    let (block, trailer) = bytes.split_first_chunk::<FIXED_TAIL>()?;
    let (&month_year, rest) = block.split_first_chunk::<8>()?;
    let (&time, _) = rest.split_first_chunk::<8>()?;
    let (_, &second_zone) = block.split_last_chunk::<8>()?;
    let month_year = MONTH_YEAR.digits(month_year)?;
    let time = TIME.digits(time)?;
    let second_zone = SECOND_ZONE.digits(second_zone)?;
    let &[.., sign @ (b'+' | b'-'), _, _, _, _] = block else {
        return None;
    };
    let (month, _) = month_name(block)?;
    numeric_zone_end(trailer)?;
    Some(date_time(
        [
            day,
            0,
            lanes(month_year, 4, 4),
            lanes(time, 1, 2),
            lanes(time, 4, 2),
            lanes(second_zone, 0, 2),
            lanes(second_zone, 4, 4),
        ],
        month,
        sign == b'-',
    ))
}

struct Layout {
    template: u64,
    fixed: u64,
    digits: u64,
}

impl Layout {
    const fn new(pattern: &[u8; 8]) -> Layout {
        let mut layout = Layout {
            template: 0,
            fixed: 0,
            digits: 0,
        };
        let mut rest: &[u8] = pattern;
        let mut shift = 0;
        while let [byte, tail @ ..] = rest {
            match *byte {
                b'D' => {
                    layout.template |= (b'0' as u64) << shift;
                    layout.digits |= 0xff << shift;
                }
                b'?' => {}
                other => {
                    layout.template |= (other as u64) << shift;
                    layout.fixed |= 0xff << shift;
                }
            }
            shift += 8;
            rest = tail;
        }
        layout
    }

    fn digits(&self, word: [u8; 8]) -> Option<u64> {
        let diff = u64::from_le_bytes(word) ^ self.template;
        let values = diff & self.digits;
        let out_of_range = (values.wrapping_add(DIGIT_CEILING) | values) & self.digits & HIGH_BITS;
        (diff & self.fixed == 0 && out_of_range == 0).then_some(values)
    }
}

fn lanes(word: u64, first: u32, count: u32) -> u32 {
    (first..first + count).fold(0, |value, lane| {
        value * 10 + ((word >> (lane * 8)) & 0xff) as u32
    })
}

fn two_digits(tens: u8, units: u8) -> u32 {
    u32::from(tens - b'0') * 10 + u32::from(units - b'0')
}

fn time(bytes: &[u8]) -> Option<(u32, u32, Option<u32>, &[u8])> {
    let (hour, rest) = number(bytes, 2)?;
    let (minute, rest) = number(rest.strip_prefix(b":")?, 2)?;
    match rest {
        [b':', rest @ ..] => {
            let (second, rest) = number(rest, 2)?;
            Some((hour, minute, Some(second), rest))
        }
        _ => Some((hour, minute, None, rest)),
    }
}

fn skip_space(mut bytes: &[u8]) -> &[u8] {
    while let [b' ' | b'\t' | b'\r', rest @ ..] | [b'\n', b' ' | b'\t', rest @ ..] = bytes {
        bytes = rest;
    }
    bytes
}

fn separator(mut bytes: &[u8]) -> Option<&[u8]> {
    let mut closed = false;
    loop {
        match bytes {
            [b' ' | b'\t', rest @ ..] | [b'\n', b' ' | b'\t', rest @ ..] => {
                closed = true;
                bytes = rest;
            }
            [b'\r', rest @ ..] => bytes = rest,
            _ => return closed.then_some(bytes),
        }
    }
}

fn skip_day_name(bytes: &[u8]) -> Option<&[u8]> {
    match bytes {
        [b'0'..=b'9', ..] => Some(bytes),
        [first, second, third, b',', rest @ ..]
            if first.is_ascii_alphabetic()
                && second.is_ascii_alphabetic()
                && third.is_ascii_alphabetic() =>
        {
            Some(skip_space(rest))
        }
        [first, ..] if first.is_ascii_alphabetic() => {
            let mut rest = bytes;
            while let [letter, tail @ ..] = rest
                && letter.is_ascii_alphabetic()
            {
                rest = tail;
            }
            rest.strip_prefix(b",").map(skip_space)
        }
        _ => None,
    }
}

fn number(mut bytes: &[u8], max_digits: u32) -> Option<(u32, &[u8])> {
    let mut value = 0;
    let mut digits = 0;
    while let [digit @ b'0'..=b'9', rest @ ..] = bytes {
        digits += 1;
        if digits > max_digits {
            return None;
        }
        value = value * 10 + u32::from(digit - b'0');
        bytes = rest;
    }
    (digits > 0).then_some((value, bytes))
}

fn numeric_zone_end(bytes: &[u8]) -> Option<()> {
    match bytes {
        [] | [b'\n'] | [b'\r'] | [b'\r', b'\n'] => Some(()),
        _ => {
            separator(bytes)?;
            inert(bytes)
        }
    }
}

fn inert(bytes: &[u8]) -> Option<()> {
    let (words, rest) = bytes.as_chunks::<8>();
    let effect = words
        .iter()
        .any(|&word| has_effect(u64::from_le_bytes(word)))
        || rest.iter().any(|&byte| matches!(byte, b'+' | b'-' | b';'));
    (!effect).then_some(())
}

fn has_effect(word: u64) -> bool {
    b"+-;"
        .iter()
        .any(|&byte| has_zero_byte(word ^ (LANES * u64::from(byte))))
}

fn has_zero_byte(word: u64) -> bool {
    word.wrapping_sub(LANES) & !word & HIGH_BITS != 0
}

fn month_name(bytes: &[u8]) -> Option<(u8, &[u8])> {
    match bytes {
        [first, second, third, rest @ ..]
            if first.is_ascii_alphabetic()
                && second.is_ascii_alphabetic()
                && third.is_ascii_alphabetic() =>
        {
            let month = month_number(letter_hash(*second) + letter_hash(*third));
            (month != 0).then_some((month, rest))
        }
        _ => None,
    }
}

fn letter_hash(letter: u8) -> usize {
    LETTER_HASH
        .get(usize::from(letter.to_ascii_lowercase().wrapping_sub(b'a')))
        .map_or(31, |&hash| usize::from(hash))
}

fn month_number(hash: usize) -> u8 {
    MONTH_MAP.get(hash).copied().unwrap_or(0)
}

fn obs_key(letter: u8, bytes: &[u8]) -> ([u8; 3], &[u8]) {
    match bytes {
        [first, second, rest @ ..] => ([letter, *first, *second], rest),
        [first] => ([letter, *first, 0], &[]),
        [] => ([letter, 0, 0], &[]),
    }
}

fn obs_zone(key: [u8; 3]) -> i32 {
    match &key {
        b"EDT" => -4,
        b"EST" | b"CDT" => -5,
        b"CST" | b"MDT" => -6,
        b"MST" | b"PDT" => -7,
        b"PST" => -8,
        _ => 0,
    }
}

fn pow10(exponent: u32) -> u32 {
    POW10.get(exponent as usize).copied().unwrap_or(1)
}

fn ends_with_sign_and_digit(mut bytes: &[u8]) -> bool {
    while let [rest @ .., b'\r' | b'\n'] = bytes {
        bytes = rest;
    }
    matches!(bytes, [.., b'+' | b'-', b'0'..=b'9'])
}

fn skip_comment(iter: &mut Iter<'_, u8>) -> bool {
    let mut depth = 1usize;
    while let Some(&byte) = iter.next() {
        match byte {
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            b'(' => depth += 1,
            b'\\' => {
                if let [b')', rest @ ..] = iter.as_slice() {
                    *iter = rest.iter();
                }
            }
            b'\n' => match iter.as_slice() {
                [b' ' | b'\t', rest @ ..] => *iter = rest.iter(),
                _ => return false,
            },
            _ => {}
        }
    }
    false
}

struct Scanner {
    pos: usize,
    parts: [u32; SLOTS],
    widths: [u32; SLOTS],
    month_hash: usize,
    month_letters: usize,
    tz_before_gmt: bool,
    new_token: bool,
    ignore: bool,
}

impl Scanner {
    const START: Scanner = Scanner {
        pos: 0,
        parts: [0; SLOTS],
        widths: WIDTHS,
        month_hash: 0,
        month_letters: 0,
        tz_before_gmt: false,
        new_token: true,
        ignore: true,
    };

    fn scan(bytes: &[u8]) -> Option<DateTime> {
        let mut state = Scanner::START;
        let mut in_comment = false;
        let mut iter = bytes.iter();
        while let Some(&byte) = iter.next() {
            match byte {
                b'a'..=b'z' | b'A'..=b'Z' => {
                    state.new_token = false;
                    if state.pos == MONTH_SLOT {
                        if matches!(state.month_letters, 1 | 2) {
                            state.month_hash += letter_hash(byte);
                        }
                        state.month_letters += 1;
                    } else if state.pos == ZONE_SLOT {
                        let (key, rest) = obs_key(byte, iter.as_slice());
                        iter = rest.iter();
                        state.set_obs_zone(obs_zone(key));
                        state.close_slot();
                    } else {
                        while let [b'a'..=b'z' | b'A'..=b'Z', rest @ ..] = iter.as_slice() {
                            iter = rest.iter();
                        }
                    }
                }
                b'0'..=b'9' => state.digit(byte - b'0'),
                b' ' | b'\t' => {
                    if state.token_open() {
                        state.close_slot();
                    }
                }
                b':' => {
                    if state.token_open() && matches!(state.pos, 3 | 4) {
                        state.close_slot();
                    }
                }
                b'\n' => match iter.as_slice() {
                    [b' ' | b'\t', rest @ ..] => {
                        iter = rest.iter();
                        if state.token_open() {
                            state.close_slot();
                        }
                    }
                    _ => break,
                },
                b'(' => {
                    state.new_token = true;
                    if !skip_comment(&mut iter) {
                        in_comment = true;
                        break;
                    }
                }
                b'+' => state.pos = ZONE_SLOT,
                b'-' => {
                    state.tz_before_gmt = true;
                    state.pos = ZONE_SLOT;
                }
                b';' => state = Scanner::START,
                _ => {}
            }
        }
        state.finish(bytes, in_comment)
    }

    fn token_open(&self) -> bool {
        !self.new_token && !self.ignore
    }

    fn digit(&mut self, digit: u8) {
        if let (Some(part), Some(width)) =
            (self.parts.get_mut(self.pos), self.widths.get_mut(self.pos))
            && *width > 0
        {
            *width -= 1;
            *part += u32::from(digit) * pow10(*width);
            self.ignore = false;
        }
        self.new_token = false;
    }

    fn close_slot(&mut self) {
        if let (Some(part), Some(&width)) =
            (self.parts.get_mut(self.pos), self.widths.get(self.pos))
            && width > 0
        {
            *part /= pow10(width);
        }
        self.pos += 1;
        self.new_token = true;
    }

    fn set_obs_zone(&mut self, zone: i32) {
        self.tz_before_gmt = zone < 0;
        let [.., part] = &mut self.parts;
        *part = 100 * zone.unsigned_abs();
        let [.., width] = &mut self.widths;
        *width = 0;
    }

    fn finish(&self, bytes: &[u8], in_comment: bool) -> Option<DateTime> {
        if self.pos < ZONE_SLOT {
            return None;
        }
        let [_, month, ..] = self.parts;
        let month = if self.month_letters == 3 {
            month_number(self.month_hash)
        } else {
            month as u8
        };
        if !(1..=12).contains(&month) {
            return None;
        }
        let mut parts = self.parts;
        let [.., width] = self.widths;
        if self.pos == ZONE_SLOT && width == 3 && !in_comment && ends_with_sign_and_digit(bytes) {
            let [.., zone] = &mut parts;
            *zone /= 10;
        }
        Some(date_time(parts, month, self.tz_before_gmt))
    }
}

#[cfg(test)]
mod tests {
    use super::{Scanner, canonical};
    use crate::{DateTime, HeaderForm, fields::tests::load_tests};
    use chrono::{FixedOffset, MappedLocalTime, SecondsFormat, TimeZone, Utc};
    use serde_json::Value as Json;

    fn parse(input: &str) -> Option<DateTime> {
        HeaderForm::Date
            .parse(input.as_bytes())
            .value()
            .as_datetime()
    }

    fn expected(json: &Json) -> Option<DateTime> {
        if json.is_null() {
            return None;
        }
        let number = |name: &str| {
            json.get(name)
                .and_then(Json::as_u64)
                .unwrap_or_else(|| panic!("fixture field {name}"))
        };
        let small = |name: &str| u8::try_from(number(name)).expect("fits in u8");
        Some(DateTime {
            year: u16::try_from(number("year")).expect("fits in u16"),
            month: small("month"),
            day: small("day"),
            hour: small("hour"),
            minute: small("minute"),
            second: small("second"),
            tz_before_gmt: json
                .get("tz_before_gmt")
                .and_then(Json::as_bool)
                .expect("fixture field tz_before_gmt"),
            tz_hour: small("tz_hour"),
            tz_minute: small("tz_minute"),
        })
    }

    fn check_with_chrono(input: &str, date: &DateTime) {
        let offset = (i32::from(date.tz_hour) * 3600 + i32::from(date.tz_minute) * 60)
            * if date.tz_before_gmt { 1 } else { -1 };
        let zone = FixedOffset::west_opt(offset)
            .unwrap_or_else(|| FixedOffset::east_opt(0).expect("UTC is a valid offset"));
        if let MappedLocalTime::Single(chrono_date) | MappedLocalTime::Ambiguous(chrono_date, _) =
            zone.with_ymd_and_hms(
                i32::from(date.year),
                u32::from(date.month),
                u32::from(date.day),
                u32::from(date.hour),
                u32::from(date.minute),
                u32::from(date.second),
            )
        {
            let rfc3339 = |timestamp: i64| {
                Utc.timestamp_opt(timestamp, 0)
                    .single()
                    .map(|utc| utc.to_rfc3339_opts(SecondsFormat::Secs, true))
            };
            assert_eq!(
                chrono_date.timestamp(),
                date.to_timestamp(),
                "{} -> {} ({:?}) -> {} ({:?})",
                input.escape_debug(),
                date.to_timestamp(),
                rfc3339(date.to_timestamp()),
                chrono_date.timestamp(),
                rfc3339(chrono_date.timestamp()),
            );
            let timestamp = date.to_timestamp();
            assert_eq!(
                DateTime::from_timestamp(timestamp).to_timestamp(),
                timestamp
            );
        }
    }

    #[test]
    fn parse_dates() {
        let tests = load_tests("date.json");
        assert_eq!(tests.len(), 84);
        for (header, expected_json) in tests {
            let parsed = parse(&header);
            assert_eq!(parsed, expected(&expected_json), "failed for {header:?}");
            if let Some(date) = parsed.filter(DateTime::is_valid) {
                check_with_chrono(&header, &date);
            }
        }
    }

    #[test]
    fn datetime_to_timezone() {
        let dt = DateTime::parse_rfc3339("2021-01-01T00:00:00Z").expect("valid date");

        for (tz, expected) in [
            (0i64, "2021-01-01T00:00:00Z"),
            (3600, "2021-01-01T01:00:00+01:00"),
            (-3600, "2020-12-31T23:00:00-01:00"),
            (19800, "2021-01-01T05:30:00+05:30"),
            (-12600, "2020-12-31T20:30:00-03:30"),
            (20700, "2021-01-01T05:45:00+05:45"),
            (16200, "2021-01-01T04:30:00+04:30"),
            (34200, "2021-01-01T09:30:00+09:30"),
            (-45900, "2020-12-31T11:15:00-12:45"),
        ] {
            let converted = dt.to_timezone(tz);
            assert_eq!(converted.to_rfc3339(), expected, "failed for tz {tz}");
            assert!(converted.is_valid(), "invalid datetime for tz {tz}");
            assert_eq!(
                converted.to_timestamp(),
                dt.to_timestamp(),
                "roundtrip failed for tz {tz}"
            );
        }
    }

    #[test]
    fn parse_rfc822() {
        assert_eq!(
            DateTime::parse_rfc822("Sat, 20 Nov 2021 14:22:01 -0800").map(|d| d.to_rfc3339()),
            Some("2021-11-20T14:22:01-08:00".to_string())
        );
        assert_eq!(DateTime::parse_rfc822("not a date"), None);
        assert_eq!(DateTime::parse_rfc822(""), None);
    }

    const SAMPLES: [&str; 12] = [
        " Tue, 1 Jul 2003 10:52:37 +0200\n",
        " Fri, 07 Oct 1994 16:15:05 -0700 (PDT)\r\n",
        "Mon, 13 Aug 1998 17:42:41 +1000",
        " 26 Jan 2002 13:00:00 -0800 (PST)\n",
        " Sat, 30 Jun 2001 10:35:28 EDT\n",
        "\n\t09 Mar 2011 11:19:46 -0000\n",
        " \t Mon, 29 Oct 2001 14:47:10 -0500\t\n",
        " Wed, 27 Jun 99 04:11 +0900 \n",
        "Thu,\n   13\n  Feb\n    1969\n  23:32\n  -0330 (Newfoundland Time)\n",
        " Wed,  9 Mar 2011 06:19:47 -0500 (EST)\n",
        " Thu, 10 Jul 1997 14:53:31 Z\r\n",
        " Thu, 10 Jul 1997 14:53:31 UT\n",
    ];

    fn variants(sample: &str) -> Vec<Vec<u8>> {
        const REPLACEMENTS: &[&[u8]] = &[
            b" ", b"\t", b"\n", b"\r", b"\r\n", b"\n ", b"\r\n\t", b"(", b")", b"\\", b";", b"+",
            b"-", b":", b",", b"0", b"9", b"A", b"z", b"\x80", b"",
        ];
        const ZONES: &[&str] = &[
            "GMT",
            "UT",
            "UTC",
            "Z",
            "A",
            "M",
            "EST",
            "EDT",
            "CST",
            "CDT",
            "MST",
            "MDT",
            "PST",
            "PDT",
            "est",
            "EST5EDT",
            "+5",
            "-5",
            "+05",
            "+530",
            "-0000",
            "+2400",
            "+9999",
            "+05:30",
            "",
            "(PDT)",
            "-0700 (PDT)",
            "+0200 +EST",
            "-0700\n (PDT)",
            "PDT-7",
        ];
        let bytes = sample.as_bytes();
        let mut out = Vec::with_capacity(bytes.len() * (REPLACEMENTS.len() + 2) + ZONES.len());
        out.extend((0..=bytes.len()).filter_map(|end| bytes.get(..end).map(<[u8]>::to_vec)));
        for (index, _) in bytes.iter().enumerate() {
            for replacement in REPLACEMENTS {
                let mut variant = Vec::with_capacity(bytes.len() + replacement.len());
                variant.extend_from_slice(bytes.get(..index).unwrap_or_default());
                variant.extend_from_slice(replacement);
                variant.extend_from_slice(bytes.get(index + 1..).unwrap_or_default());
                out.push(variant);
            }
        }
        let trimmed = sample.trim_end();
        if let Some((head, _)) = trimmed.rsplit_once(' ') {
            out.extend(
                ZONES
                    .iter()
                    .map(|zone| format!("{head} {zone}\n").into_bytes()),
            );
        }
        out
    }

    #[test]
    fn fast_path_matches_scanner() {
        let mut fast = 0;
        let mut total = 0;
        let fixtures = load_tests("date.json");
        let inputs = SAMPLES
            .iter()
            .copied()
            .chain(fixtures.iter().map(|(header, _)| header.as_str()));
        for sample in inputs {
            for variant in variants(sample) {
                total += 1;
                if let Some(date) = canonical(&variant) {
                    fast += 1;
                    assert_eq!(
                        Some(date),
                        Scanner::scan(&variant),
                        "{:?}",
                        String::from_utf8_lossy(&variant)
                    );
                }
            }
        }
        assert!(fast * 10 > total, "fast path taken for {fast} of {total}");
        for sample in SAMPLES {
            assert!(canonical(sample.as_bytes()).is_some(), "{sample:?}");
        }
    }
}
