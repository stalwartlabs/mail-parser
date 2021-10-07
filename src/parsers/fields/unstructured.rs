use std::borrow::Cow;

use crate::parsers::{message_stream::MessageStream, rfc2047::Rfc2047Parser};
enum UnstructuredState {
    Lf,
    Space,
    Char,
}

fn add_token<'x>(
    mut tokens: Vec<Cow<'x, str>>,
    stream: &'x MessageStream,
    token_start: usize,
    token_end: usize,
    is_token_safe: bool,
) -> Vec<Cow<'x, str>> {
    let bytes = stream.get_bytes(token_start - 1, token_end).unwrap();

    if !tokens.is_empty() {
        tokens.push(Cow::from(" ".to_string()));
    }
    tokens.push(if is_token_safe {
        Cow::from(unsafe { std::str::from_utf8_unchecked(bytes) })
    } else {
        String::from_utf8_lossy(bytes)
    });

    tokens
}

pub fn parse_unstructured<'x>(stream: &'x MessageStream) -> Option<Cow<'x, str>> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut is_token_safe = true;
    let mut state = UnstructuredState::Space;
    let mut tokens: Vec<Cow<str>> = Vec::new();

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => {
                if let UnstructuredState::Lf = state {
                    stream.rewind(1);
                    break;
                } else if token_start > 0 {
                    tokens = add_token(tokens, stream, token_start, token_end, is_token_safe);
                    token_start = 0;
                    is_token_safe = true;
                }

                state = UnstructuredState::Lf;
            }
            b' ' | b'\t' | b'\r' => {
                state = UnstructuredState::Space;
            }
            _ => {
                if *ch > 0x7f && is_token_safe {
                    is_token_safe = false;
                }

                match state {
                    UnstructuredState::Lf => {
                        stream.rewind(1);
                        break;
                    }
                    UnstructuredState::Space => {
                        if *ch == b'=' && stream.skip_byte(&b'?') {
                            let pos_back = stream.get_pos() - 1;

                            if let Some(token) = Rfc2047Parser::parse(stream) {
                                if token_start > 0 {
                                    tokens = add_token(
                                        tokens,
                                        stream,
                                        token_start,
                                        token_end,
                                        is_token_safe,
                                    );
                                    tokens.push(Cow::from(" ".to_string()));
                                    token_start = 0;
                                    is_token_safe = true;
                                }
                                tokens.push(Cow::from(token));
                                state = UnstructuredState::Space;
                                continue;
                            } else {
                                stream.set_pos(pos_back);
                            }
                        }
                        state = UnstructuredState::Char;

                        if token_start == 0 {
                            token_start = stream.get_pos();
                            token_end = stream.get_pos()
                        }
                    }
                    UnstructuredState::Char => token_end = stream.get_pos(),
                }
            }
        }
    }

    match tokens.len() {
        1 => tokens.pop(),
        0 => None,
        _ => Some(Cow::from(tokens.concat())),
    }
}

mod tests {
    use std::borrow::Cow;

    use crate::parsers::{fields::unstructured::parse_unstructured, message_stream::MessageStream};

    #[test]
    fn test_unstructured() {
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
