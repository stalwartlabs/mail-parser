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

use std::borrow::Cow;

use crate::{decoders::encoded_word::decode_rfc2047, parsers::message::MessageStream, HeaderValue};
struct UnstructuredParser<'x> {
    token_start: usize,
    token_end: usize,
    is_token_start: bool,
    tokens: Vec<Cow<'x, str>>,
}

fn add_token<'x>(parser: &mut UnstructuredParser<'x>, stream: &MessageStream<'x>, add_space: bool) {
    if parser.token_start > 0 {
        if !parser.tokens.is_empty() {
            parser.tokens.push(" ".into());
        }
        parser.tokens.push(String::from_utf8_lossy(
            &stream.data[parser.token_start - 1..parser.token_end],
        ));

        if add_space {
            parser.tokens.push(" ".into());
        }

        parser.token_start = 0;
        parser.is_token_start = true;
    }
}

pub fn parse_unstructured<'x>(stream: &mut MessageStream<'x>) -> HeaderValue<'x> {
    let mut parser = UnstructuredParser {
        token_start: 0,
        token_end: 0,
        is_token_start: true,
        tokens: Vec::new(),
    };

    let mut iter = stream.data[stream.pos..].iter();

    while let Some(ch) = iter.next() {
        stream.pos += 1;
        match ch {
            b'\n' => {
                add_token(&mut parser, stream, false);

                match stream.data.get(stream.pos) {
                    Some(b' ' | b'\t') => {
                        if !parser.is_token_start {
                            parser.is_token_start = true;
                        }
                        iter.next();
                        stream.pos += 1;
                        continue;
                    }
                    _ => {
                        return match parser.tokens.len() {
                            1 => HeaderValue::Text(parser.tokens.pop().unwrap()),
                            0 => HeaderValue::Empty,
                            _ => HeaderValue::Text(parser.tokens.concat().into()),
                        };
                    }
                }
            }
            b' ' | b'\t' => {
                if !parser.is_token_start {
                    parser.is_token_start = true;
                }
                continue;
            }
            b'=' if parser.is_token_start => {
                if let (bytes_read, Some(token)) = decode_rfc2047(stream, stream.pos) {
                    add_token(&mut parser, stream, true);
                    parser.tokens.push(token.into());
                    stream.pos += bytes_read;
                    iter = stream.data[stream.pos..].iter();
                    continue;
                }
            }
            b'\r' => continue,
            _ => (),
        }

        if parser.is_token_start {
            parser.is_token_start = false;
        }

        if parser.token_start == 0 {
            parser.token_start = stream.pos;
        }

        parser.token_end = stream.pos;
    }

    HeaderValue::Empty
}

pub fn thread_name(text: &str) -> &str {
    static THREAD_PREFIXES: &[&str] = &[
        "re",
        "fwd",
        "fw",
        "sv",
        "vs",
        "antw",
        "doorst",
        "vl",
        "ref",
        "tr",
        "aw",
        "wg",
        "απ",
        "πρθ",
        "השב",
        "הועבר",
        "vá",
        "továbbítás",
        "r",
        "rif",
        "i",
        "fs",
        "bls",
        "trs",
        "vb",
        "rv",
        "res",
        "enc",
        "odp",
        "pd",
        "ynt",
        "i̇lt",
        "atb",
        "yml",
        "رد",
        "إعادة توجيه",
        "回复",
        "回覆",
        "转发",
        "轉寄",
    ];

    let mut token_start = 0;
    let mut token_end = 0;

    let mut in_brackets = false;
    let mut token_found = false;

    for (pos, ch) in text.char_indices() {
        match ch {
            ':' => {
                if in_brackets || !token_found {
                    break;
                }
                if token_end == 0 {
                    token_end = pos;
                }
                if THREAD_PREFIXES.contains(&text[token_start..token_end].to_lowercase().as_ref()) {
                    token_found = false;
                    token_start = pos + 1;
                } else {
                    break;
                }
            }
            '[' => {
                if !in_brackets && token_found {
                    token_end = pos;
                    in_brackets = true;
                } else {
                    break;
                }
            }
            ']' => {
                if in_brackets {
                    in_brackets = false;
                } else {
                    break;
                }
            }
            _ if ch.is_ascii_digit() => {
                if !in_brackets {
                    break;
                }
            }
            _ if ch.is_whitespace() => {
                if token_found || in_brackets {
                    break;
                }
            }
            _ if ch.is_alphabetic() => {
                if !token_found {
                    token_start = pos;
                    token_end = 0;
                    token_found = true;
                } else if in_brackets {
                    break;
                }
            }
            _ => break,
        }
    }
    &text[token_start..]
}

#[cfg(test)]
mod tests {
    use crate::parsers::fields::unstructured::thread_name;

    #[test]
    fn parse_thread_name() {
        let tests = [
            ("re: hello", "hello"),
            ("re:re: hello", "hello"),
            ("re:fwd: hello", "hello"),
            ("fwd[5]:re[5]: hello", "hello"),
            ("fwd[99]:  re[40]: hello", "hello"),
            (": hello", ": hello"),
            ("z: hello", "z: hello"),
            ("re:: hello", ": hello"),
            ("[10] hello", "[10] hello"),
            ("fwd[a]: hello", "fwd[a]: hello"),
            ("re:", ""),
            ("re::", ":"),
            ("回复: 轉寄: 轉寄", "轉寄"),
            ("aw[50]: wg: aw[1]: hallo", "hallo"),
            ("res: rv: enc: továbbítás: ", " "),
        ];

        for (input, expected) in tests {
            assert_eq!(thread_name(input), expected, "{:?}", input);
        }
    }

    #[test]
    fn parse_unstructured_text() {
        use crate::parsers::fields::unstructured::parse_unstructured;
        use crate::parsers::message::MessageStream;

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
        ];

        for input in inputs {
            let str = input.0.to_string();
            assert_eq!(
                parse_unstructured(&mut MessageStream::new(str.as_bytes()),).unwrap_text(),
                input.1,
                "Failed to parse '{:?}'",
                input.0
            );
        }
    }
}
