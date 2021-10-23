use std::borrow::Cow;

use crate::{decoders::encoded_word::parse_encoded_word, parsers::message_stream::MessageStream};
struct UnstructuredParser<'x> {
    token_start: usize,
    token_end: usize,
    is_token_safe: bool,
    is_token_start: bool,
    tokens: Vec<Cow<'x, str>>,
}

fn add_token<'x>(parser: &mut UnstructuredParser<'x>, stream: &MessageStream<'x>, add_space: bool) {
    if parser.token_start > 0 {
        if !parser.tokens.is_empty() {
            parser.tokens.push(" ".into());
        }
        parser.tokens.push(
            stream
                .get_string(
                    parser.token_start - 1,
                    parser.token_end,
                    parser.is_token_safe,
                )
                .unwrap(),
        );

        if add_space {
            parser.tokens.push(" ".into());
        }

        parser.token_start = 0;
        parser.is_token_safe = true;
        parser.is_token_start = true;
    }
}

pub fn parse_unstructured<'x>(stream: &MessageStream<'x>) -> Option<Cow<'x, str>> {
    let mut parser = UnstructuredParser {
        token_start: 0,
        token_end: 0,
        is_token_safe: true,
        is_token_start: true,
        tokens: Vec::new(),
    };

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => {
                add_token(&mut parser, stream, false);

                match stream.peek() {
                    Some(b' ' | b'\t') => {
                        if !parser.is_token_start {
                            parser.is_token_start = true;
                        }
                        stream.advance(1);
                        continue;
                    }
                    _ => {
                        return match parser.tokens.len() {
                            1 => parser.tokens.pop().unwrap().into(),
                            0 => None,
                            _ => Some(parser.tokens.concat().into()),
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
                if let Some(token) = parse_encoded_word(stream) {
                    add_token(&mut parser, stream, true);
                    parser.tokens.push(token.into());
                    continue;
                }
            }
            b'\r' => continue,
            0..=0x7f => (),
            _ => {
                if parser.is_token_safe {
                    parser.is_token_safe = false;
                }
            }
        }

        if parser.is_token_start {
            parser.is_token_start = false;
        }

        if parser.token_start == 0 {
            parser.token_start = stream.get_pos();
        }

        parser.token_end = stream.get_pos();
    }

    None
}

mod tests {

    #[test]
    fn parse_unstructured_text() {
        use crate::parsers::{
            fields::unstructured::parse_unstructured, message_stream::MessageStream,
        };
        use std::borrow::Cow;

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
            let mut str = input.0.to_string();
            assert_eq!(
                parse_unstructured(&MessageStream::new(unsafe { str.as_bytes_mut() }),).unwrap(),
                input.1,
                "Failed to parse '{:?}'",
                input.0
            );
        }
    }
}
