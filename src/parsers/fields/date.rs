use std::fmt;

use crate::parsers::message_stream::MessageStream;

#[derive(PartialEq)]
pub struct DateTime {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    tz_hour: u32,
    tz_minute: u32,
    tz_before_gmt: bool,
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

pub fn parse_date(stream: &MessageStream, abort_on_invalid: bool) -> Option<DateTime> {
    let mut pos = 0;
    let mut parts: [u32; 7] = [0; 7];
    let mut parts_sizes: [u32; 7] = [
        2, // Day (0)
        2, // Month (1)
        4, // Year (2)
        2, // Hour (3)
        2, // Minute (4)
        2, // Second (5)
        4, // TZ (6)
    ];
    let mut month: [u8; 3] = [0; 3];
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
                    if !is_new_token && !ignore {
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
                }
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
                if pos == 1 && month_pos < 3 {
                    month[month_pos] = if *ch <= b'Z' { *ch + 32 } else { *ch };
                    month_pos += 1;
                }
                if is_new_token {
                    is_new_token = false;
                }
            }
            b'(' => {
                comment_count += 1;
                is_new_token = true;
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

    if pos < 6 {
        return None;
    }

    if month_pos == 3 {
        parts[1] = match &month {
            b"jan" => 1,
            b"feb" => 2,
            b"mar" => 3,
            b"apr" => 4,
            b"may" => 5,
            b"jun" => 6,
            b"jul" => 7,
            b"aug" => 8,
            b"sep" => 9,
            b"oct" => 10,
            b"nov" => 11,
            b"dec" => 12,
            _ => 0,
        }
    }

    if (1..=99).contains(&parts[2]) {
        parts[2] += 1900;
    }

    Some(DateTime {
        year: parts[2],
        month: parts[1],
        day: parts[0],
        hour: parts[3],
        minute: parts[4],
        second: parts[5],
        tz_hour: parts[6] / 100,
        tz_minute: parts[6] % 100,
        tz_before_gmt: !is_plus,
    })
}

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
            ("21 Nov 97 09:55:06 GMT", "1997-11-21T09:55:06+00:00"),
            (
                "20 11 (some \n 79 comments(more comments\n )) 79 05:34:27 -0300",
                "1979-11-20T05:34:27-03:00",
            ),
            (" Wed, 27 Jun 99 04:11 +0900 ", "1999-06-27T04:11:00+09:00"),
        ];

        for input in inputs {
            let result = parse_date(&MessageStream::new(input.0.as_bytes()), false)
                .unwrap()
                .to_iso8601();

            println!("{} -> {}", input.0.escape_debug(), result);
            assert_eq!(input.1, result);
        }
    }
}
