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

use crate::{
    decoders::{
        base64::Base64Decoder, charsets::map::get_charset_decoder,
        quoted_printable::QuotedPrintableDecoder,
    },
    parsers::message_stream::MessageStream,
};

use super::charsets::map::decoder_default;

pub fn parse_encoded_word<'x>(stream: &MessageStream<'x>) -> Option<Cow<'x, str>> {
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

    let (success, is_utf8_safe, bytes) = match stream.next() {
        Some(b'q') | Some(b'Q') if stream.skip_byte(&b'?') => {
            stream.decode_quoted_printable(b"?=", true)
        }
        Some(b'b') | Some(b'B') if stream.skip_byte(&b'?') => stream.decode_base64(b"?=", true),
        _ => (false, false, None),
    };

    if !success {
        stream.set_pos(backup_pos);
        return None;
    }

    if let Some(decoder) =
        get_charset_decoder(stream.get_bytes(charset_start, charset_end).unwrap())
    {
        decoder(bytes?).into()
    } else if is_utf8_safe {
        // SAFE: slice checked previously
        unsafe { Cow::from(std::str::from_utf8_unchecked(bytes?)).into() }
    } else {
        decoder_default(bytes?).into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        decoders::encoded_word::parse_encoded_word, parsers::message_stream::MessageStream,
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
            #[cfg(feature = "full_encoding")]
            (
                "?shift_jis?B?g26DjYFbgUWDj4Fbg4uDaA==?=".to_string(),
                "ハロー・ワールド",
                true,
            ),
            #[cfg(feature = "full_encoding")]
            (
                "?iso-2022-jp?q?=1B$B%O%m!<!&%o!<%k%I=1B(B?=".to_string(),
                "ハロー・ワールド",
                true,
            ),
        ];

        for input in inputs {
            let mut str = input.0.to_string();

            match parse_encoded_word(&MessageStream::new(unsafe { str.as_bytes_mut() })) {
                Some(string) => {
                    //println!("Decoded '{}'", string);
                    assert_eq!(string, input.1);
                }
                None => panic!("Failed to decode '{}'", input.0),
            }
        }
    }
}
