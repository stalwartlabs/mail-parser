use std::borrow::Cow;

use crate::{
    decoders::{
        base64::Base64Decoder, charsets::parser::CharsetParser,
        quoted_printable::QuotedPrintableDecoder, Decoder, DecoderResult,
    },
    parsers::message_stream::MessageStream,
};

enum Rfc2047Parser {
    Charset,
    Language,
    Encoding,
    Data,
}

impl Rfc2047Parser {
    pub fn parse<'x>(stream: &'x mut MessageStream) -> Option<Cow<'x, str>> {
        let mut ch_parser = CharsetParser::new();
        let mut ch_decoder = None;
        let mut decoder: Option<Box<dyn Decoder>> = None;
        let mut state = Rfc2047Parser::Charset;
    
        while let Some(ch) = stream.next() {
            match state {
                Rfc2047Parser::Charset => match ch {
                    b'?' => {
                        ch_decoder = ch_parser.get_decoder(75);
                        state = Rfc2047Parser::Encoding;
                    }
                    b'*' => {
                        ch_decoder = ch_parser.get_decoder(75);
                        state = Rfc2047Parser::Language;
                    }
                    b' '..=b'z' => ch_parser.ingest(*ch),
                    _ => return None,
                },
                Rfc2047Parser::Language => {
                    // Ignore language
                    match ch {
                        b'?' => state = Rfc2047Parser::Encoding,
                        b'\n' | b'=' => return None,
                        _ => ()
                    }
                }
                Rfc2047Parser::Encoding => {
                    if decoder.is_none() {
                        match ch {
                            b'q' | b'Q' => decoder = Some(Box::new(QuotedPrintableDecoder::new(true))),
                            b'b' | b'B' => decoder = Some(Box::new(Base64Decoder::new())),
                            _ => return None,
                        }
                    } else if *ch == b'?' {
                        state = Rfc2047Parser::Data;
                        if ch_decoder.is_none() {
                            stream.begin_write();
                        }
                    } else {
                        return None;
                    }
                }
                Rfc2047Parser::Data => match ch {
                    b'?' => {
                        stream.skip_byte(b'=');

                        return if let Some(ref mut ch_decoder) = ch_decoder {
                            ch_decoder
                                .to_string()
                                .map(Cow::from)
                        } else {
                            stream
                                .end_write()
                                .map(String::from_utf8_lossy)
                        };
                    }
                    b'\n' => return None,
                    _ => {
                        match decoder.as_mut().unwrap().ingest(ch) {
                            DecoderResult::Byte(b) => {
                                if let Some(ref mut ch_decoder) = ch_decoder {
                                    ch_decoder.ingest(&b);
                                } else {
                                    stream.write(b);
                                }
                            },
                            DecoderResult::ByteArray(ba) => {
                                if let Some(ref mut ch_decoder) = ch_decoder {
                                    ch_decoder.ingest_slice(ba);
                                } else {
                                    stream.write_slice(ba);
                                }                                
                            },
                            DecoderResult::NeedData => (),
                            DecoderResult::Error => return None,
                        }
                    }
                },
            }
        }
    
        None
    }
}


#[cfg(test)]
mod tests {
    use crate::parsers::message_stream::MessageStream;
    use super::Rfc2047Parser;

    #[test]
    fn decode_rfc2047() {
        unsafe {
            let inputs = [
                ("iso-8859-1?q?this=20is=20some=20text?=".to_string(), "this is some text", true),
                ("iso-8859-1?q?this is some text?=".to_string(), "this is some text", true),
                ("US-ASCII?Q?Keith_Moore?=".to_string(), "Keith Moore", false),
                ("ISO-8859-1?Q?Keld_J=F8rn_Simonsen?=".to_string(), "Keld Jørn Simonsen", true),
                ("ISO-8859-1?B?SWYgeW91IGNhbiByZWFkIHRoaXMgeW8=?=".to_string(), "If you can read this yo", true),
                ("ISO-8859-2?B?dSB1bmRlcnN0YW5kIHRoZSBleGFtcGxlLg==?=".to_string(), "u understand the example.", true),
                ("ISO-8859-1?Q?Olle_J=E4rnefors?=".to_string(), "Olle Järnefors", true),
                ("ISO-8859-1?Q?Patrik_F=E4ltstr=F6m?=".to_string(), "Patrik Fältström", true),
                ("ISO-8859-1?Q?a?=".to_string(), "a", true),
                ("ISO-8859-1?Q?a_b?=".to_string(), "a b", true),
                ("utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=".to_string(), "Thís ís válíd ÚTF8", false),
                ("utf-8?q?Th=C3=ADs_=C3=ADs_v=C3=A1l=C3=ADd_=C3=9ATF8?=".to_string(), "Thís ís válíd ÚTF8", false),
                ("Iso-8859-6?Q?=E5=D1=CD=C8=C7 =C8=C7=E4=D9=C7=E4=E5?=".to_string(), "مرحبا بالعالم", true),
                ("Iso-8859-6?b?5dHNyMcgyMfk2cfk5Q==?=".to_string(), "مرحبا بالعالم", true),
                #[cfg(feature = "multibytedecode")]
                ("shift_jis?B?g26DjYFbgUWDj4Fbg4uDaA==?=".to_string(), "ハロー・ワールド", true),
                #[cfg(feature = "multibytedecode")]
                ("iso-2022-jp?q?=1B$B%O%m!<!&%o!<%k%I=1B(B?=".to_string(), "ハロー・ワールド", true),
                ];

            for input in inputs {
                match Rfc2047Parser::parse(&mut MessageStream::new(input.0.clone().as_bytes_mut())) {
                    Some(string) => {
                        match string {
                            std::borrow::Cow::Borrowed(string) => {
                                //println!("Decoded '{}' -> Borrowed", string);
                                assert_eq!(string, input.1);
                                assert!(!input.2);
                            },
                            std::borrow::Cow::Owned(string) => {
                                //println!("Decoded '{}' -> Owned", string);
                                assert_eq!(string, input.1);
                                assert!(input.2);
                            },
                        }
                    },
                    None => panic!("Failed to parse '{}'", input.0),
                }
            }
        }
    }
}
