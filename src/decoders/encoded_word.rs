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

use crate::{decoders::charsets::map::get_charset_decoder, parsers::message::MessageStream};

use super::{
    base64::decode_base64_word, quoted_printable::decode_quoted_printable_word, DecodeWordFnc,
};

enum Rfc2047State {
    Init,
    Charset,
    Encoding,
    Data,
}

pub fn decode_rfc2047(stream: &MessageStream, start_pos: usize) -> (usize, Option<String>) {
    let mut read_pos: usize = start_pos;
    let mut state = Rfc2047State::Init;

    let mut charset_start = 0;
    let mut charset_end = 0;
    let mut decode_fnc: Option<DecodeWordFnc> = None;

    for ch in stream.data[start_pos..].iter() {
        read_pos += 1;

        match state {
            Rfc2047State::Init => {
                if ch != &b'?' {
                    return (0, None);
                }
                state = Rfc2047State::Charset;
                charset_start = read_pos;
                charset_end = read_pos;
            }
            Rfc2047State::Charset => match ch {
                b'?' => {
                    if charset_end == charset_start {
                        charset_end = read_pos - 1;
                    }
                    if (charset_end - charset_start) < 2 {
                        return (0, None);
                    }
                    state = Rfc2047State::Encoding;
                }
                b'*' => {
                    if charset_end == charset_start {
                        charset_end = read_pos - 1;
                    }
                }
                b'\n' => {
                    return (0, None);
                }
                _ => (),
            },
            Rfc2047State::Encoding => {
                match ch {
                    b'q' | b'Q' => decode_fnc = Some(decode_quoted_printable_word),
                    b'b' | b'B' => decode_fnc = Some(decode_base64_word),
                    _ => {
                        return (0, None);
                    }
                }
                state = Rfc2047State::Data;
            }
            Rfc2047State::Data => {
                if ch != &b'?' {
                    return (0, None);
                } else {
                    break;
                }
            }
        }
    }

    if let (Some(decode_fnc), Some(enc_bytes)) = (decode_fnc, stream.data.get(read_pos..)) {
        let (bytes_read, bytes) = decode_fnc(enc_bytes);
        if bytes_read != usize::MAX {
            return (
                (read_pos - start_pos) + bytes_read,
                if let Some(decoder) = get_charset_decoder(&stream.data[charset_start..charset_end])
                {
                    decoder(&bytes).into()
                } else {
                    String::from_utf8(bytes)
                        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
                        .into()
                },
            );
        }
    }

    (0, None)
}

#[cfg(test)]
mod tests {
    use crate::parsers::message::MessageStream;

    #[test]
    fn decode_rfc2047() {
        for (input, expected_result, _) in [
            (
                "?iso-8859-1?q?this=20is=20some=20text?=",
                "this is some text",
                true,
            ),
            (
                "?iso-8859-1?q?this is some text?=",
                "this is some text",
                true,
            ),
            ("?US-ASCII?Q?Keith_Moore?=", "Keith Moore", false),
            (
                "?iso_8859-1:1987?Q?Keld_J=F8rn_Simonsen?=",
                "Keld Jørn Simonsen",
                true,
            ),
            (
                "?ISO-8859-1?B?SWYgeW91IGNhbiByZWFkIHRoaXMgeW8=?=",
                "If you can read this yo",
                true,
            ),
            (
                "?ISO-8859-2?B?dSB1bmRlcnN0YW5kIHRoZSBleGFtcGxlLg==?=",
                "u understand the example.",
                true,
            ),
            ("?ISO-8859-1?Q?Olle_J=E4rnefors?=", "Olle Järnefors", true),
            (
                "?ISO-8859-1?Q?Patrik_F=E4ltstr=F6m?=",
                "Patrik Fältström",
                true,
            ),
            ("?ISO-8859-1*?Q?a?=", "a", true),
            ("?ISO-8859-1**?Q?a_b?=", "a b", true),
            (
                "?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=",
                "Thís ís válíd ÚTF8",
                false,
            ),
            (
                "?utf-8*unknown?q?Th=C3=ADs_=C3=ADs_v=C3=A1l=C3=ADd_=C3=9ATF8?=",
                "Thís ís válíd ÚTF8",
                false,
            ),
            (
                "?Iso-8859-6?Q?=E5=D1=CD=C8=C7 =C8=C7=E4=D9=C7=E4=E5?=",
                "مرحبا بالعالم",
                true,
            ),
            (
                "?Iso-8859-6*arabic?b?5dHNyMcgyMfk2cfk5Q==?=",
                "مرحبا بالعالم",
                true,
            ),
            #[cfg(feature = "full_encoding")]
            (
                "?shift_jis?B?g26DjYFbgUWDj4Fbg4uDaA==?=",
                "ハロー・ワールド",
                true,
            ),
            #[cfg(feature = "full_encoding")]
            (
                "?iso-2022-jp?q?=1B$B%O%m!<!&%o!<%k%I=1B(B?=",
                "ハロー・ワールド",
                true,
            ),
        ] {
            match super::decode_rfc2047(&MessageStream::new(input.as_bytes()), 0) {
                (_, Some(result)) => {
                    //println!("Decoded '{}'", string);
                    assert_eq!(result, expected_result);
                }
                _ => panic!("Failed to decode '{}'", input),
            }
        }
    }
}
