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

use std::borrow::Cow;

use crate::{parsers::MessageStream, HeaderValue};
struct UnstructuredParser<'x> {
    token_start: usize,
    token_end: usize,
    tokens: Vec<Cow<'x, str>>,
}

impl<'x> UnstructuredParser<'x> {
    fn add_token(&mut self, stream: &MessageStream<'x>, add_space: bool) {
        if self.token_start > 0 {
            if !self.tokens.is_empty() {
                self.tokens.push(" ".into());
            }
            self.tokens.push(String::from_utf8_lossy(
                stream.bytes(self.token_start - 1..self.token_end),
            ));

            if add_space {
                self.tokens.push(" ".into());
            }

            self.token_start = 0;
        }
    }
}

impl<'x> MessageStream<'x> {
    pub fn parse_unstructured(&mut self) -> HeaderValue<'x> {
        let mut parser = UnstructuredParser {
            token_start: 0,
            token_end: 0,
            tokens: Vec::new(),
        };

        while let Some(ch) = self.next() {
            match ch {
                b'\n' => {
                    parser.add_token(self, false);

                    if !self.try_next_is_space() {
                        return match parser.tokens.len() {
                            1 => HeaderValue::Text(parser.tokens.pop().unwrap()),
                            0 => HeaderValue::Empty,
                            _ => HeaderValue::Text(parser.tokens.concat().into()),
                        };
                    } else {
                        continue;
                    }
                }
                b' ' | b'\t' | b'\r' => {
                    continue;
                }
                b'=' if self.peek_char(b'?') => {
                    self.checkpoint();
                    if let Some(token) = self.decode_rfc2047() {
                        parser.add_token(self, true);
                        parser.tokens.push(token.into());
                        continue;
                    }
                    self.restore();
                }
                _ => (),
            }

            if parser.token_start == 0 {
                parser.token_start = self.offset();
            }

            parser.token_end = self.offset();
        }

        HeaderValue::Empty
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::MessageStream;

    #[test]
    fn parse_unstructured() {
        let inputs = [
            ("Saying Hello\n", "Saying Hello", true),
            ("Re: Saying Hello\r\n", "Re: Saying Hello", true),
            (" Fwd: \n\tSaying \n Hello\r\n", "Fwd: Saying Hello", false),
            (
                " FWD: \n Saying Hello \nX-Mailer: 123\r\n",
                "FWD: Saying Hello",
                false,
            ),
            (
                concat!(
                    " from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600\n"
                ),
                concat!(
                    "from x.y.test by example.net via TCP with ESMTP id ABC12345",
                    " for <mary@example.net>;  21 Nov 1997 10:05:43 -0600"
                ),
                false,
            ),
            (
                "=?iso-8859-1?q?this is some text?=\n",
                "this is some text",
                false,
            ),
            (
                "=?iso-8859-1?q?this=20is=20some=20text?=\r\n",
                "this is some text",
                false,
            ),
            (
                concat!(
                    " =?ISO-8859-1?B?SWYgeW91IGNhbiByZWFkIHRoaXMgeW8=?=\n     ",
                    "=?ISO-8859-2?B?dSB1bmRlcnN0YW5kIHRoZSBleGFtcGxlLg==?=\n"
                ),
                "If you can read this you understand the example.",
                false,
            ),
            (" =?ISO-8859-1?Q?a?=\n", "a", false),
            ("=?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?b?=\n", "ab", false),
            ("=?ISO-8859-1?Q?a?=  =?ISO-8859-1?Q?b?=\n", "ab", false),
            (
                "=?ISO-8859-1?Q?a?=\r\n    =?ISO-8859-1?Q?b?=\nFrom: unknown@domain.com\n",
                "ab",
                false,
            ),
            ("=?ISO-8859-1?Q?a_b?=\n", "a b", false),
            ("=?ISO-8859-1?Q?a?= =?ISO-8859-2?Q?_b?=\r\n", "a b", false),
            (
                concat!(
                    " this =?iso-8859-1?q?is?= some =?iso-8859-1?q?t?=\n =?iso-8859-1?q?e?=",
                    " \n =?iso-8859-1?q?x?=\n =?iso-8859-1?q?t?=\n"
                ),
                "this is some text",
                false,
            ),
            (" =\n", "=", true),
            (" =? \n", "=?", true),
            ("=?utf-8 \n", "=?utf-8", true),
            ("=?utf-8? \n", "=?utf-8?", true),
            (
                " let's = try =?iso-8859-1? to break\n =? the \n parser\n",
                "let's = try =?iso-8859-1? to break =? the parser",
                false,
            ),
            ("ハロー・ワールド \n", "ハロー・ワールド", true),
            (
                "[SUSPECTED SPAM]=?utf-8?B?VGhpcyBpcyB0aGUgb3JpZ2luYWwgc3ViamVjdA==?=\n",
                "[SUSPECTED SPAM] This is the original subject",
                true,
            ),
            ("Some text =?utf-8?Q??=here\n", "Some text  here", true),
            (
                "=?ISO-8859-1?Q?a?==?ISO-8859-1?Q?b?==?ISO-8859-1?Q?c?= =?ISO-8859-1?Q?d?=\n",
                "abcd",
                true,
            ),
            ("=?utf-8?Q?Hello\n _there!?=\n", "Hello there!", true),
            ("=?utf-8?Q?Hello\r\n _there!?=\r\n", "Hello there!", true),
            (
                "=?utf-8?Q?Hello\r\n   \t  _there!?=\r\n",
                "Hello there!",
                true,
            ),
            (
                "[SUSPECTED SPAM]=?utf-8?B?VGhpcyBpcyB0aGUgb\n 3JpZ2luYWwgc3ViamVjdA==?=\n",
                "[SUSPECTED SPAM] This is the original subject",
                true,
            ),
            (
                "[SUSPECTED SPAM]=?utf-8?B?VGhpcyBpcyB0aGUgb\r\n 3JpZ2luYWwgc3ViamVjdA==?=\r\n",
                "[SUSPECTED SPAM] This is the original subject",
                true,
            ),
        ];

        for (input, expected_result, _) in inputs {
            assert_eq!(
                MessageStream::new(input.as_bytes())
                    .parse_unstructured()
                    .unwrap_text(),
                expected_result,
                "Failed to parse '{:?}'",
                input
            );
        }
    }
}
