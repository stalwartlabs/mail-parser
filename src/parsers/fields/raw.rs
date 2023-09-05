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

use crate::{parsers::MessageStream, HeaderValue};

impl<'x> MessageStream<'x> {
    pub fn parse_raw(&mut self) -> HeaderValue<'x> {
        let mut token_start: usize = 0;
        let mut token_end: usize = 0;

        while let Some(ch) = self.next() {
            match ch {
                b'\n' => {
                    if !self.try_next_is_space() {
                        return if token_start > 0 {
                            HeaderValue::Text(String::from_utf8_lossy(
                                self.bytes(token_start - 1..token_end),
                            ))
                        } else {
                            HeaderValue::Empty
                        };
                    } else {
                        continue;
                    }
                }
                b' ' | b'\t' | b'\r' => continue,
                _ => (),
            }

            if token_start == 0 {
                token_start = self.offset();
            }

            token_end = self.offset();
        }

        HeaderValue::Empty
    }

    pub fn parse_and_ignore(&mut self) {
        while let Some(&ch) = self.next() {
            if ch == b'\n' && !self.try_next_is_space() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{parsers::MessageStream, MessageParser};

    #[test]
    fn parse_raw_text() {
        let inputs = [
            ("Saying Hello\nMessage-Id", "Saying Hello"),
            ("Re: Saying Hello\r\n \r\nFrom:", "Re: Saying Hello"),
            (
                concat!(
                    " from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600\n"
                ),
                concat!(
                    "from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600"
                ),
            ),
        ];

        for (input, expected) in inputs {
            assert_eq!(
                MessageStream::new(input.as_bytes())
                    .parse_raw()
                    .unwrap_text(),
                expected,
                "Failed for '{:?}'",
                input
            );
        }
    }

    #[test]
    fn ordered_raw_headers() {
        let input = br#"From: Art Vandelay <art@vandelay.com>
To: jane@example.com
Date: Sat, 20 Nov 2021 14:22:01 -0800
Subject: Why not both importing AND exporting? =?utf-8?b?4pi6?=
Content-Type: multipart/mixed; boundary="festivus";

Here's a message body.
"#;
        let message = MessageParser::default().parse(input).unwrap();
        let mut iter = message.headers_raw();
        assert_eq!(
            iter.next().unwrap(),
            ("From", " Art Vandelay <art@vandelay.com>\n")
        );
        assert_eq!(iter.next().unwrap(), ("To", " jane@example.com\n"));
        assert_eq!(
            iter.next().unwrap(),
            ("Date", " Sat, 20 Nov 2021 14:22:01 -0800\n")
        );
        assert_eq!(
            iter.next().unwrap(),
            (
                "Subject",
                " Why not both importing AND exporting? =?utf-8?b?4pi6?=\n"
            )
        );
        assert_eq!(
            iter.next().unwrap(),
            ("Content-Type", " multipart/mixed; boundary=\"festivus\";\n")
        );
    }
}
