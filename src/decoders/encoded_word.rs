use crate::{
    decoders::{
        base64::Base64Decoder,
        charsets::map::{get_charset_decoder, get_default_decoder},
        quoted_printable::QuotedPrintableDecoder,
    },
    parsers::message_stream::MessageStream,
};

use super::buffer_writer::BufferWriter;

pub fn parse_encoded_word<'x>(
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) -> Option<&'x str> {
    if !stream.skip_byte(&b'?') {
        return None;
    }
    let charset_start = stream.get_pos();
    let mut charset_end = charset_start;
    let backup_pos = charset_start - 1;

    while let Some(ch) = stream.next() {
        match ch {
            b'?' => {
                if charset_end == charset_start {
                    charset_end = stream.get_pos() - 1;
                }
                break;
            }
            b'*' => {
                if charset_end == charset_start {
                    charset_end = stream.get_pos() - 1;
                }
            }
            b'\n' => {
                stream.set_pos(backup_pos);
                return None;
            }
            _ => (),
        }
    }

    if !(2..=45).contains(&(charset_end - charset_start)) {
        stream.set_pos(backup_pos);
        return None;
    }

    let mut decoder = get_charset_decoder(
        stream.get_bytes(charset_start, charset_end).unwrap(),
        buffer.get_buf_mut()?,
    )
    .unwrap_or_else(|| get_default_decoder(buffer.get_buf_mut().unwrap_or_else(|| &mut [])));

    if !(match stream.next() {
        Some(b'q') | Some(b'Q') if stream.skip_byte(&b'?') => {
            stream.decode_quoted_printable(b"?=", true, decoder.as_mut())
        }
        Some(b'b') | Some(b'B') if stream.skip_byte(&b'?') => {
            stream.decode_base64(b"?=", true, decoder.as_mut())
        }
        _ => false,
    }) {
        stream.set_pos(backup_pos);
        return None;
    }

    if decoder.len() > 0 {
        buffer.advance_tail(decoder.len());
        buffer.get_string()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        decoders::{buffer_writer::BufferWriter, encoded_word::parse_encoded_word},
        parsers::message_stream::MessageStream,
    };

    #[test]
    fn decode_rfc2047() {
        let inputs = [
            (
                "?iso-8859-1?q?this=20is=20some=20text?=".to_string(),
                "this is some text",
                true,
            ),
            (
                "?iso-8859-1?q?this is some text?=".to_string(),
                "this is some text",
                true,
            ),
            (
                "?US-ASCII?Q?Keith_Moore?=".to_string(),
                "Keith Moore",
                false,
            ),
            (
                "?iso_8859-1:1987?Q?Keld_J=F8rn_Simonsen?=".to_string(),
                "Keld Jørn Simonsen",
                true,
            ),
            (
                "?ISO-8859-1?B?SWYgeW91IGNhbiByZWFkIHRoaXMgeW8=?=".to_string(),
                "If you can read this yo",
                true,
            ),
            (
                "?ISO-8859-2?B?dSB1bmRlcnN0YW5kIHRoZSBleGFtcGxlLg==?=".to_string(),
                "u understand the example.",
                true,
            ),
            (
                "?ISO-8859-1?Q?Olle_J=E4rnefors?=".to_string(),
                "Olle Järnefors",
                true,
            ),
            (
                "?ISO-8859-1?Q?Patrik_F=E4ltstr=F6m?=".to_string(),
                "Patrik Fältström",
                true,
            ),
            ("?ISO-8859-1*?Q?a?=".to_string(), "a", true),
            ("?ISO-8859-1**?Q?a_b?=".to_string(), "a b", true),
            (
                "?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=".to_string(),
                "Thís ís válíd ÚTF8",
                false,
            ),
            (
                "?utf-8*unknown?q?Th=C3=ADs_=C3=ADs_v=C3=A1l=C3=ADd_=C3=9ATF8?=".to_string(),
                "Thís ís válíd ÚTF8",
                false,
            ),
            (
                "?Iso-8859-6?Q?=E5=D1=CD=C8=C7 =C8=C7=E4=D9=C7=E4=E5?=".to_string(),
                "مرحبا بالعالم",
                true,
            ),
            (
                "?Iso-8859-6*arabic?b?5dHNyMcgyMfk2cfk5Q==?=".to_string(),
                "مرحبا بالعالم",
                true,
            ),
            #[cfg(feature = "multibytedecode")]
            (
                "?shift_jis?B?g26DjYFbgUWDj4Fbg4uDaA==?=".to_string(),
                "ハロー・ワールド",
                true,
            ),
            #[cfg(feature = "multibytedecode")]
            (
                "?iso-2022-jp?q?=1B$B%O%m!<!&%o!<%k%I=1B(B?=".to_string(),
                "ハロー・ワールド",
                true,
            ),
        ];

        for input in inputs {
            match parse_encoded_word(
                &MessageStream::new(input.0.as_bytes()),
                &BufferWriter::new(&mut BufferWriter::alloc_buffer(input.0.len() * 2)),
            ) {
                Some(string) => {
                    //println!("Decoded '{}'", string);
                    assert_eq!(string, input.1);
                }
                None => panic!("Failed to decode '{}'", input.0),
            }
        }
    }
}
