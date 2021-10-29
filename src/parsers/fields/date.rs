/*
 * Copyright Stalwart Labs, Minter Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::fmt;

use crate::parsers::message_stream::MessageStream;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    tz_before_gmt: bool,
    tz_hour: u32,
    tz_minute: u32,
}

impl DateTime {
    pub fn to_iso8601(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:{:02}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            if self.tz_before_gmt { "-" } else { "+" },
            self.tz_hour,
            self.tz_minute
        )
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.to_iso8601())
    }
}

pub fn parse_date<'x>(stream: &'x MessageStream, abort_on_invalid: bool) -> Option<DateTime> {
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

    while let Some(ch) = stream.next() {
        let mut next_part = false;

        match ch {
            b'\n' => match stream.peek() {
                Some(b' ' | b'\t') => {
                    stream.advance(1);
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
                    stream.skip_byte(&b')');
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
                        month_hash += unsafe {
                            *MONTH_HASH
                                .get_unchecked((if *ch <= b'Z' { *ch + 32 } else { *ch }) as usize)
                        } as usize;
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
            _ => {
                if abort_on_invalid {
                    stream.rewind(1);
                    break;
                }
            }
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
        Some(DateTime {
            year: if (1..=99).contains(&parts[2]) {
                parts[2] + 1900
            } else {
                parts[2]
            },
            month: if month_pos == 3 && month_hash <= 30 {
                (unsafe { *MONTH_MAP.get_unchecked(month_hash) }) as u32
            } else {
                parts[1]
            },
            day: parts[0],
            hour: parts[3],
            minute: parts[4],
            second: parts[5],
            tz_hour: parts[6] / 100,
            tz_minute: parts[6] % 100,
            tz_before_gmt: !is_plus,
        })
    } else {
        None
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

pub static MONTH_MAP: &[u8] = &[
    5, 0, 0, 0, 10, 3, 0, 0, 0, 7, 1, 0, 0, 0, 12, 6, 0, 0, 0, 8, 4, 0, 0, 0, 2, 9, 0, 0, 0, 0, 11,
];

mod tests {
    use crate::parsers::{fields::date::parse_date, message_stream::MessageStream};

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
            ("21 Nov 97 09:55:06 GMT", "1997-11-21T09:55:06+00:00"),
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

        for input in inputs {
            let mut str = input.0.to_string();
            match parse_date(&MessageStream::new(unsafe { str.as_bytes_mut() }), false) {
                Some(date) => {
                    //println!("{} -> {}", input.0.escape_debug(), date.to_iso8601());
                    assert_eq!(input.1, date.to_iso8601());
                }
                None => {
                    //println!("{} -> None", input.0.escape_debug());
                    assert!(input.1.is_empty());
                }
                _ => unreachable!(),
            }
        }
    }
}
