/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::fmt;

use crate::{parsers::message::MessageStream, DateTime, HeaderValue};

impl DateTime {
    /// Parses an RFC822 date
    pub fn parse_rfc822(&self, value: &str) -> Option<Self> {
        match parse_date(&mut MessageStream {
            data: value.as_bytes(),
            pos: 0,
        }) {
            HeaderValue::DateTime(dt) => dt.into(),
            _ => None,
        }
    }

    /// Returns an RFC3339 representation of the parsed RFC5322 datetime field
    pub fn to_rfc3339(&self) -> String {
        if self.tz_hour != 0 || self.tz_minute != 0 {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:{:02}",
                self.year,
                self.month,
                self.day,
                self.hour,
                self.minute,
                self.second,
                if self.tz_before_gmt && (self.tz_hour > 0 || self.tz_minute > 0) {
                    "-"
                } else {
                    "+"
                },
                self.tz_hour,
                self.tz_minute
            )
        } else {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                self.year, self.month, self.day, self.hour, self.minute, self.second,
            )
        }
    }

    /// Returns true if the date is valid
    pub fn is_valid(&self) -> bool {
        (0..=23).contains(&self.tz_hour)
            && (1970..=3000).contains(&self.year)
            && (0..=59).contains(&self.tz_minute)
            && (1..=12).contains(&self.month)
            && (1..=31).contains(&self.day)
            && (0..=23).contains(&self.hour)
            && (0..=59).contains(&self.minute)
            && (0..=59).contains(&self.second)
    }

    /// Returns the numbers of seconds since 1970-01-01T00:00:00Z (Unix epoch)
    /// or None if the date is invalid.
    pub fn to_timestamp(&self) -> i64 {
        // Ported from https://github.com/protocolbuffers/upb/blob/22182e6e/upb/json_decode.c#L982-L992
        let month = self.month as u32;
        let year_base = 4800; /* Before min year, multiple of 400. */
        let m_adj = month.wrapping_sub(3); /* March-based month. */
        let carry = if m_adj > month { 1 } else { 0 };
        let adjust = if carry > 0 { 12 } else { 0 };
        let y_adj = self.year as i64 + year_base - carry;
        let month_days = ((m_adj.wrapping_add(adjust)) * 62719 + 769) / 2048;
        let leap_days = y_adj / 4 - y_adj / 100 + y_adj / 400;
        (y_adj * 365 + leap_days + month_days as i64 + (self.day as i64 - 1) - 2472632) * 86400
            + self.hour as i64 * 3600
            + self.minute as i64 * 60
            + self.second as i64
            + ((self.tz_hour as i64 * 3600 + self.tz_minute as i64 * 60)
                * if self.tz_before_gmt { 1 } else { -1 })
    }
}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cmp(other).into()
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.to_timestamp() - other.to_timestamp() {
            0 => std::cmp::Ordering::Equal,
            x if x > 0 => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Less,
        }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.to_rfc3339())
    }
}

pub fn parse_date<'x>(stream: &mut MessageStream<'x>) -> HeaderValue<'x> {
    let mut pos = 0;
    let mut parts = [0u32; 7];
    let mut parts_sizes = [
        2u32, // Day (0)
        2u32, // Month (1)
        4u32, // Year (2)
        2u32, // Hour (3)
        2u32, // Minute (4)
        2u32, // Second (5)
        4u32, // TZ (6)
    ];
    let mut month_hash: usize = 0;
    let mut month_pos: usize = 0;

    let mut is_plus = true;
    let mut is_new_token = true;
    let mut ignore = true;
    let mut comment_count = 0;

    let mut iter = stream.data[stream.pos..].iter();

    while let Some(ch) = iter.next() {
        let mut next_part = false;
        stream.pos += 1;

        match ch {
            b'\n' => match stream.data.get(stream.pos) {
                Some(b' ' | b'\t') => {
                    iter.next();
                    stream.pos += 1;

                    if !is_new_token && !ignore && comment_count == 0 {
                        next_part = true;
                    } else {
                        continue;
                    }
                }
                _ => break,
            },
            _ if comment_count > 0 => {
                if *ch == b')' {
                    comment_count -= 1;
                } else if *ch == b'(' {
                    comment_count += 1;
                } else if *ch == b'\\' {
                    if let Some(b')') = stream.data.get(stream.pos) {
                        stream.pos += 1;
                    }
                }
                continue;
            }
            b'0'..=b'9' => {
                if pos < 7 && parts_sizes[pos] > 0 {
                    parts_sizes[pos] -= 1;
                    parts[pos] += (*ch - b'0') as u32 * u32::pow(10, parts_sizes[pos]);

                    if ignore {
                        ignore = false;
                    }
                }
                if is_new_token {
                    is_new_token = false;
                }
            }
            b':' => {
                if !is_new_token && !ignore && (pos == 3 || pos == 4) {
                    next_part = true;
                }
            }
            b'+' => {
                pos = 6;
            }
            b'-' => {
                is_plus = false;
                pos = 6;
            }
            b' ' | b'\t' => {
                if !is_new_token && !ignore {
                    next_part = true;
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                if pos == 1 {
                    if (1..=2).contains(&month_pos) {
                        month_hash += MONTH_HASH
                            [(if *ch <= b'Z' { *ch + 32 } else { *ch }) as usize]
                            as usize;
                    }
                    month_pos += 1;
                }
                if is_new_token {
                    is_new_token = false;
                }
            }
            b'(' => {
                comment_count += 1;
                is_new_token = true;
                continue;
            }
            b',' | b'\r' => (),
            _ => (),
        }

        if next_part {
            if pos < 7 && parts_sizes[pos] > 0 {
                parts[pos] /= u32::pow(10, parts_sizes[pos]);
            }
            pos += 1;
            is_new_token = true;
        }
    }

    if pos >= 6 {
        HeaderValue::DateTime(DateTime {
            year: if (1..=99).contains(&parts[2]) {
                parts[2] + 1900
            } else {
                parts[2]
            } as u16,
            month: if month_pos == 3 && month_hash <= 30 {
                MONTH_MAP[month_hash] as u8
            } else {
                parts[1] as u8
            },
            day: parts[0] as u8,
            hour: parts[3] as u8,
            minute: parts[4] as u8,
            second: parts[5] as u8,
            tz_hour: (parts[6] / 100) as u8,
            tz_minute: (parts[6] % 100) as u8,
            tz_before_gmt: !is_plus,
        })
    } else {
        HeaderValue::Empty
    }
}

static MONTH_HASH: &[u8] = &[
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 0, 14, 4, 31, 10, 31, 14, 31, 31, 31, 31, 4, 31, 10, 15, 15, 31, 5, 31, 0, 5, 15, 31, 31,
    0, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
];

pub static MONTH_MAP: &[u8; 31] = &[
    5, 0, 0, 0, 10, 3, 0, 0, 0, 7, 1, 0, 0, 0, 12, 6, 0, 0, 0, 8, 4, 0, 0, 0, 2, 9, 0, 0, 0, 0, 11,
];

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, LocalResult, SecondsFormat, TimeZone, Utc};

    use crate::{
        parsers::{fields::date::parse_date, message::MessageStream},
        HeaderValue,
    };

    #[test]
    fn parse_dates() {
        let inputs = [
            (
                "Fri, 21 Nov 1997 09:55:06 -0600",
                "1997-11-21T09:55:06-06:00",
            ),
            (
                "Tue, 1 Jul 2003 10:52:37 +0200",
                "2003-07-01T10:52:37+02:00",
            ),
            (
                "Thu, 13 Feb 1969 23:32:54 -0330",
                "1969-02-13T23:32:54-03:30",
            ),
            (
                "Mon, 24 Nov 1997 14:22:01 -0800",
                "1997-11-24T14:22:01-08:00",
            ),
            (
                "Thu,\n   13\n  Feb\n    1969\n  23:32\n  -0330 (Newfoundland Time)\n",
                "1969-02-13T23:32:00-03:30",
            ),
            (
                "Tue, 1 Jul 2003 10:52:37 +0200",
                "2003-07-01T10:52:37+02:00",
            ),
            (
                " 1 Jul 2003 (comment about date) 10:52:37 +0200",
                "2003-07-01T10:52:37+02:00",
            ),
            (
                "Tue, 1 Jul 2003 ((tricky)\n comment) 10:52:37 +0200",
                "2003-07-01T10:52:37+02:00",
            ),
            ("21 Nov 97 09:55:06 GMT", "1997-11-21T09:55:06Z"),
            (
                "20 11 (some \n 44 comments(more comments\n )) 79 05:34:27 -0300",
                "1979-11-20T05:34:27-03:00",
            ),
            (" Wed, 27 Jun 99 04:11 +0900 ", "1999-06-27T04:11:00+09:00"),
            (
                " 4 8 15 16 23 42, 4 8 15 16 23 42, 4 8 15 16 23 42, ",
                "1915-08-04T16:23:42+00:04",
            ),
            (" some numbers 0 1 2 but invalid ", ""),
            ("Tue, 1 Jul 2003 ((invalid)\ncomment) 10:52:37 +0200", ""),
            ("1 jan 2021 09:55:06 +0200", "2021-01-01T09:55:06+02:00"),
            ("2 feb 2021 09:55:06 +0200", "2021-02-02T09:55:06+02:00"),
            ("3 mar 2021 09:55:06 +0200", "2021-03-03T09:55:06+02:00"),
            ("4 apr 2021 09:55:06 +0200", "2021-04-04T09:55:06+02:00"),
            ("5 may 2021 09:55:06 +0200", "2021-05-05T09:55:06+02:00"),
            ("6 jun 2021 09:55:06 +0200", "2021-06-06T09:55:06+02:00"),
            ("7 jul 2021 09:55:06 +0200", "2021-07-07T09:55:06+02:00"),
            ("8 aug 2021 09:55:06 +0200", "2021-08-08T09:55:06+02:00"),
            ("9 sep 2021 09:55:06 +0200", "2021-09-09T09:55:06+02:00"),
            ("10 oct 2021 09:55:06 +0200", "2021-10-10T09:55:06+02:00"),
            ("11 nov 2021 09:55:06 +0200", "2021-11-11T09:55:06+02:00"),
            ("12 dec 2021 09:55:06 +0200", "2021-12-12T09:55:06+02:00"),
            ("13 zzz 2021 09:55:06 +0200", "2021-00-13T09:55:06+02:00"),
        ];

        for (input, expected_result) in inputs {
            match parse_date(&mut MessageStream::new(input.as_bytes())) {
                HeaderValue::DateTime(datetime) => {
                    assert_eq!(expected_result, datetime.to_rfc3339());

                    if datetime.is_valid() {
                        if let LocalResult::Single(chrono_datetime)
                        | LocalResult::Ambiguous(chrono_datetime, _) = FixedOffset::west_opt(
                            ((datetime.tz_hour as i32 * 3600i32) + datetime.tz_minute as i32 * 60)
                                * if datetime.tz_before_gmt { 1i32 } else { -1i32 },
                        )
                        .unwrap_or_else(|| FixedOffset::east(0))
                        .ymd_opt(
                            datetime.year as i32,
                            datetime.month as u32,
                            datetime.day as u32,
                        )
                        .and_hms_opt(
                            datetime.hour as u32,
                            datetime.minute as u32,
                            datetime.second as u32,
                        ) {
                            assert_eq!(
                                chrono_datetime.timestamp(),
                                datetime.to_timestamp(),
                                "{} -> {} ({}) -> {} ({})",
                                input.escape_debug(),
                                datetime.to_timestamp(),
                                Utc.timestamp_opt(datetime.to_timestamp(), 0)
                                    .unwrap()
                                    .to_rfc3339_opts(SecondsFormat::Secs, true),
                                chrono_datetime.timestamp(),
                                Utc.timestamp_opt(chrono_datetime.timestamp(), 0)
                                    .unwrap()
                                    .to_rfc3339_opts(SecondsFormat::Secs, true)
                            );
                        }
                    }
                }
                HeaderValue::Empty => {
                    //println!("{} -> None", input.0.escape_debug());
                    assert!(expected_result.is_empty());
                }
                _ => panic!("Unexpected result"),
            }
        }
    }
}
