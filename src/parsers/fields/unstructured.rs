use std::borrow::Cow;

use crate::parsers::{encoded_word::parse_encoded_word, message_stream::MessageStream};
pub struct UnstructuredParser<'x> {
    token_start: usize,
    token_end: usize,
    is_token_safe: bool,
    is_token_start: bool,
    tokens: Vec<Cow<'x, str>>,
}

impl<'x> UnstructuredParser<'x> {
    pub fn new() -> UnstructuredParser<'x> {
        UnstructuredParser {
            token_start: 0,
            token_end: 0,
            is_token_safe: true,
            is_token_start: true,
            tokens: Vec::new(),
        }
    }
}

pub fn add_token<'x>(
    mut parser: UnstructuredParser<'x>,
    stream: &'x MessageStream,
) -> UnstructuredParser<'x> {
    let bytes = stream
        .get_bytes(parser.token_start - 1, parser.token_end)
        .unwrap();

    if !parser.tokens.is_empty() {
        parser.tokens.push(Cow::from(" "));
    }
    parser.tokens.push(if parser.is_token_safe {
        Cow::from(unsafe { std::str::from_utf8_unchecked(bytes) })
    } else {
        parser.is_token_safe = true;
        String::from_utf8_lossy(bytes)
    });

    parser.token_start = 0;
    parser.is_token_start = true;
    parser
}

pub fn parse_unstructured<'x>(stream: &'x MessageStream) -> Option<Cow<'x, str>> {
    let mut parser = UnstructuredParser::new();

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }
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
                            1 => parser.tokens.pop(),
                            0 => None,
                            _ => Some(Cow::from(parser.tokens.concat())),
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
            b'=' if parser.is_token_start && stream.skip_byte(&b'?') => {
                let pos_back = stream.get_pos() - 1;

                if let Some(token) = parse_encoded_word(stream) {
                    if parser.token_start > 0 {
                        parser = add_token(parser, stream);
                        parser.tokens.push(Cow::from(" "));
                    }
                    parser.tokens.push(Cow::from(token));
                    continue;
                } else {
                    stream.set_pos(pos_back);
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
    use std::borrow::Cow;

    use crate::parsers::{fields::unstructured::parse_unstructured, message_stream::MessageStream};

    #[test]
    fn parse_unstructured_text() {
        let inputs = [
            ("Saying Hello\n".to_string(), "Saying Hello", true),
            ("Re: Saying Hello\r\n".to_string(), "Re: Saying Hello", true),
            (" Fwd: \n\tSaying \n Hello\r\n".to_string(), "Fwd: Saying Hello", false),
            (" FWD: \n Saying Hello \nX-Mailer: 123\r\n".to_string(), "FWD: Saying Hello", false),
            (" from x.y.test\n      by example.net\n      via TCP\n      with ESMTP\n      id ABC12345\n      for <mary@example.net>;  21 Nov 1997 10:05:43 -0600\n".to_string(), "from x.y.test by example.net via TCP with ESMTP id ABC12345 for <mary@example.net>;  21 Nov 1997 10:05:43 -0600", false),
            ("=?iso-8859-1?q?this is some text?=\n".to_string(), "this is some text", false),
            ("=?iso-8859-1?q?this=20is=20some=20text?=\r\n".to_string(), "this is some text", false),
            (" =?ISO-8859-1?B?SWYgeW91IGNhbiByZWFkIHRoaXMgeW8=?=\n     =?ISO-8859-2?B?dSB1bmRlcnN0YW5kIHRoZSBleGFtcGxlLg==?=\n".to_string(), "If you can read this you understand the example.", false),
            (" =?ISO-8859-1?Q?a?=\n".to_string(), "a", false),
            ("=?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?b?=\n".to_string(), "ab", false),
            ("=?ISO-8859-1?Q?a?=  =?ISO-8859-1?Q?b?=\n".to_string(), "ab", false),
            ("=?ISO-8859-1?Q?a?=\r\n    =?ISO-8859-1?Q?b?=\nFrom: unknown@domain.com\n".to_string(), "ab", false),
            ("=?ISO-8859-1?Q?a_b?=\n".to_string(), "a b", false),
            ("=?ISO-8859-1?Q?a?= =?ISO-8859-2?Q?_b?=\r\n".to_string(), "a b", false),
            (" this =?iso-8859-1?q?is?= some =?iso-8859-1?q?t?=\n =?iso-8859-1?q?e?= \n =?iso-8859-1?q?x?=\n =?iso-8859-1?q?t?=\n".to_string(), "this is some text", false),
            (" =\n".to_string(), "=", true),
            (" =? \n".to_string(), "=?", true),
            ("=?utf-8 \n".to_string(), "=?utf-8", true),
            ("=?utf-8? \n".to_string(), "=?utf-8?", true),
            (" let's = try =?iso-8859-1? to break\n =? the \n parser\n".to_string(), "let's = try =?iso-8859-1? to break =? the parser", false),
            ("ハロー・ワールド \n".to_string(), "ハロー・ワールド", true),
        ];

        for input in inputs {
            match parse_unstructured(&MessageStream::new(input.0.as_bytes())) {
                Some(cow) => {
                    assert_eq!(cow, input.1);
                    if let Cow::Borrowed(_) = cow {
                        assert!(
                            input.2,
                            "Expected Owned but received Borrowed string '{}'.",
                            cow
                        );
                        //println!("'{}' -> Borrowed", cow);
                    } else {
                        assert!(
                            !input.2,
                            "Expected Borrowed but received Owned string '{}'.",
                            cow
                        );
                        //println!("'{}' -> Owned", cow);
                    }
                }
                None => panic!("Failed to parse '{}'", input.0),
            }
        }
    }
}
