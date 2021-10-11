use crate::{
    decoders::{
        base64::Base64Decoder,
        charsets::{parser::CharsetParser, utf8::Utf8Decoder, CharsetDecoder},
        quoted_printable::QuotedPrintableDecoder,
        Decoder, DecoderResult,
    },
    parsers::message_stream::MessageStream,
};

pub enum Rfc2047Parser {
    Charset,
    Language,
    Encoding,
}

pub fn parse_encoded_word(stream: &MessageStream) -> Option<String> {
    let mut ch_parser = CharsetParser::new();
    let mut ch_decoder: Option<Box<dyn CharsetDecoder>> = None;
    let mut decoder: Option<Box<dyn Decoder>> = None;
    let mut state = Rfc2047Parser::Charset;

    while let Some(ch) = stream.next() {
        match state {
            Rfc2047Parser::Charset => match ch {
                b'?' => {
                    ch_decoder = Some(
                        ch_parser
                            .get_decoder(75)
                            .unwrap_or_else(|| Box::new(Utf8Decoder::new(75))),
                    );
                    state = Rfc2047Parser::Encoding;
                }
                b'*' => {
                    ch_decoder = Some(
                        ch_parser
                            .get_decoder(75)
                            .unwrap_or_else(|| Box::new(Utf8Decoder::new(75))),
                    );
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
                    _ => (),
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
                    break;
                } else {
                    return None;
                }
            }
        }
    }

    // Unwrap decoders and proceed to decode data
    let mut ch_decoder = ch_decoder?;
    let mut decoder = decoder?;

    while let Some(ch) = stream.next() {
        match ch {
            b'?' => {
                stream.skip_byte(&b'=');
                return ch_decoder.to_string();
            }
            b'\n' => return None,
            _ => match decoder.ingest(ch) {
                DecoderResult::Byte(b) => ch_decoder.ingest(&b),
                DecoderResult::ByteArray(ba) => ch_decoder.ingest_slice(ba),
                DecoderResult::NeedData => (),
                DecoderResult::Error => return None,
            },
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::parsers::{encoded_word::parse_encoded_word, message_stream::MessageStream};

    #[test]
    fn decode_rfc2047() {
        let inputs = [
            (
                "iso-8859-1?q?this=20is=20some=20text?=".to_string(),
                "this is some text",
                true,
            ),
            (
                "iso-8859-1?q?this is some text?=".to_string(),
                "this is some text",
                true,
            ),
            ("US-ASCII?Q?Keith_Moore?=".to_string(), "Keith Moore", false),
            (
                "iso_8859-1:1987?Q?Keld_J=F8rn_Simonsen?=".to_string(),
                "Keld Jørn Simonsen",
                true,
            ),
            (
                "ISO-8859-1?B?SWYgeW91IGNhbiByZWFkIHRoaXMgeW8=?=".to_string(),
                "If you can read this yo",
                true,
            ),
            (
                "ISO-8859-2?B?dSB1bmRlcnN0YW5kIHRoZSBleGFtcGxlLg==?=".to_string(),
                "u understand the example.",
                true,
            ),
            (
                "ISO-8859-1?Q?Olle_J=E4rnefors?=".to_string(),
                "Olle Järnefors",
                true,
            ),
            (
                "ISO-8859-1?Q?Patrik_F=E4ltstr=F6m?=".to_string(),
                "Patrik Fältström",
                true,
            ),
            ("ISO-8859-1*?Q?a?=".to_string(), "a", true),
            ("ISO-8859-1**?Q?a_b?=".to_string(), "a b", true),
            (
                "utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=".to_string(),
                "Thís ís válíd ÚTF8",
                false,
            ),
            (
                "utf-8*unknown?q?Th=C3=ADs_=C3=ADs_v=C3=A1l=C3=ADd_=C3=9ATF8?=".to_string(),
                "Thís ís válíd ÚTF8",
                false,
            ),
            (
                "Iso-8859-6?Q?=E5=D1=CD=C8=C7 =C8=C7=E4=D9=C7=E4=E5?=".to_string(),
                "مرحبا بالعالم",
                true,
            ),
            (
                "Iso-8859-6*arabic?b?5dHNyMcgyMfk2cfk5Q==?=".to_string(),
                "مرحبا بالعالم",
                true,
            ),
            #[cfg(feature = "multibytedecode")]
            (
                "shift_jis?B?g26DjYFbgUWDj4Fbg4uDaA==?=".to_string(),
                "ハロー・ワールド",
                true,
            ),
            #[cfg(feature = "multibytedecode")]
            (
                "iso-2022-jp?q?=1B$B%O%m!<!&%o!<%k%I=1B(B?=".to_string(),
                "ハロー・ワールド",
                true,
            ),
        ];

        for input in inputs {
            match parse_encoded_word(&MessageStream::new(input.0.as_bytes())) {
                Some(string) => {
                    //println!("Decoded '{}'", string);
                    assert_eq!(string, input.1);
                }
                None => panic!("Failed to parse '{}'", input.0),
            }
        }
    }
}
