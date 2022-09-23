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

use crate::parsers::message::MessageStream;

#[derive(PartialEq, Debug)]
enum QuotedPrintableState {
    None,
    Eq,
    Hex1,
}

pub fn decode_quoted_printable(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(bytes.len());

    let mut state = QuotedPrintableState::None;
    let mut hex1 = 0;

    for &ch in bytes {
        match ch {
            b'=' => {
                if let QuotedPrintableState::None = state {
                    state = QuotedPrintableState::Eq
                } else {
                    return None;
                }
            }
            b'\n' => {
                if QuotedPrintableState::Eq == state {
                    state = QuotedPrintableState::None;
                } else {
                    buf.push(b'\n');
                }
            }
            b'\r' => (),
            _ => match state {
                QuotedPrintableState::None => {
                    buf.push(ch);
                }
                QuotedPrintableState::Eq => {
                    hex1 = HEX_MAP[ch as usize];
                    if hex1 != -1 {
                        state = QuotedPrintableState::Hex1;
                    } else {
                        return None;
                    }
                }
                QuotedPrintableState::Hex1 => {
                    let hex2 = HEX_MAP[ch as usize];

                    state = QuotedPrintableState::None;
                    if hex2 != -1 {
                        buf.push(((hex1 as u8) << 4) | hex2 as u8);
                    } else {
                        return None;
                    }
                }
            },
        }
    }

    buf.into()
}

pub fn decode_quoted_printable_mime<'x>(
    stream: &mut MessageStream<'x>,
    boundary: &[u8],
) -> (usize, Cow<'x, [u8]>) {
    let mut buf = Vec::with_capacity(stream.data.len() - stream.pos);

    let mut state = QuotedPrintableState::None;
    let mut hex1 = 0;

    let mut iter = stream.data[stream.pos..].iter().peekable();
    let mut last_ch = 0;
    let mut pos = stream.pos;
    let mut end_pos = stream.pos;

    while let Some(&ch) = iter.next() {
        pos += 1;

        match ch {
            b'=' => {
                if let QuotedPrintableState::None = state {
                    state = QuotedPrintableState::Eq
                } else {
                    return (usize::MAX, b""[..].into());
                }
            }
            b'\n' => {
                end_pos = if last_ch == b'\r' { pos - 2 } else { pos - 1 };
                if QuotedPrintableState::Eq == state {
                    state = QuotedPrintableState::None;
                } else {
                    buf.push(b'\n');
                }
            }
            b'\r' => (),
            b'-' if !boundary.is_empty()
                && matches!(iter.peek(), Some(b'-'))
                && stream.data.get(pos + 1..pos + 1 + boundary.len()) == Some(boundary)
                && matches!(
                    stream.data.get(pos + 1 + boundary.len()..),
                    Some([b'\n' | b'\r' | b' ' | b'\t', ..])
                        | Some([b'-', b'-', ..])
                        | Some([])
                        | None
                ) =>
            {
                if last_ch == b'\n' {
                    buf.pop();
                } else {
                    end_pos = pos - 1;
                }
                stream.pos = pos + boundary.len() + 1;
                buf.shrink_to_fit();
                return (end_pos, buf.into());
            }
            _ => match state {
                QuotedPrintableState::None => {
                    buf.push(ch);
                }
                QuotedPrintableState::Eq => {
                    hex1 = HEX_MAP[ch as usize];
                    if hex1 != -1 {
                        state = QuotedPrintableState::Hex1;
                    } else {
                        return (usize::MAX, b""[..].into());
                    }
                }
                QuotedPrintableState::Hex1 => {
                    let hex2 = HEX_MAP[ch as usize];

                    state = QuotedPrintableState::None;
                    if hex2 != -1 {
                        buf.push(((hex1 as u8) << 4) | hex2 as u8);
                    } else {
                        return (usize::MAX, b""[..].into());
                    }
                }
            },
        }

        last_ch = ch;
    }

    buf.shrink_to_fit();
    (
        if boundary.is_empty() {
            stream.pos = pos;
            pos
        } else {
            usize::MAX
        },
        buf.into(),
    )
}

pub fn decode_quoted_printable_word(bytes: &[u8]) -> (usize, Vec<u8>) {
    let mut bytes_read = 0;

    let mut buf = Vec::with_capacity(64);

    let mut state = QuotedPrintableState::None;
    let mut hex1 = 0;

    let mut iter = bytes.iter().peekable();

    while let Some(&ch) = iter.next() {
        bytes_read += 1;

        match ch {
            b'=' => {
                if let QuotedPrintableState::None = state {
                    state = QuotedPrintableState::Eq
                } else {
                    break;
                }
            }
            b'?' => {
                if let Some(b'=') = iter.peek() {
                    return (bytes_read + 1, buf);
                } else {
                    buf.push(b'?');
                }
            }
            b'\n' => {
                if let Some(b' ' | b'\t') = iter.peek() {
                    loop {
                        iter.next();
                        bytes_read += 1;
                        if !matches!(iter.peek(), Some(b' ' | b'\t')) {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            b'_' => {
                buf.push(b' ');
            }
            b'\r' => (),
            _ => match state {
                QuotedPrintableState::None => {
                    buf.push(ch);
                }
                QuotedPrintableState::Eq => {
                    hex1 = HEX_MAP[ch as usize];
                    if hex1 != -1 {
                        state = QuotedPrintableState::Hex1;
                    } else {
                        // Failed
                        break;
                    }
                }
                QuotedPrintableState::Hex1 => {
                    let hex2 = HEX_MAP[ch as usize];

                    state = QuotedPrintableState::None;
                    if hex2 != -1 {
                        buf.push(((hex1 as u8) << 4) | hex2 as u8);
                    } else {
                        // Failed
                        break;
                    }
                }
            },
        }
    }

    (usize::MAX, buf)
}

#[cfg(test)]
mod tests {
    use crate::parsers::message::MessageStream;

    #[test]
    fn decode_quoted_printable() {
        for (encoded_str, expected_result) in [
            (
                concat!(
                    "J'interdis aux marchands de vanter trop leurs marchandises. ",
                    "Car ils se font=\nvite p=C3=A9dagogues et t'enseignent comme but ce ",
                    "qui n'est par essence qu=\n'un moyen, et te trompant ainsi sur la route ",
                    "=C3=A0 suivre les voil=C3=\n=A0 bient=C3=B4t qui te d=C3=A9gradent, car ",
                    "si leur musique est vulgaire il=\ns te fabriquent pour te la vendre une ",
                    "=C3=A2me vulgaire.\n=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry, ",
                    "Citadelle (1948)"
                ),
                concat!(
                    "J'interdis aux marchands de vanter trop leurs marchandises. ",
                    "Car ils se fontvite pédagogues et t'enseignent comme but ce qui ",
                    "n'est par essence qu'un moyen, et te trompant ainsi sur la route ",
                    "à suivre les voilà bientôt qui te dégradent, car si leur musique ",
                    "est vulgaire ils te fabriquent pour te la vendre une âme vulgaire.\n",
                    "— Antoine de Saint-Exupéry, Citadelle (1948)"
                ),
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry",
                "— Antoine de Saint-Exupéry",
            ),
            (
                concat!(
                    "Die Hasen klagten einst uber ihre Lage; \"wir ",
                    "leben\", sprach ein=\r\n Redner, \"in steter Furcht vor Menschen",
                    " und Tieren, eine Beute der Hunde,=\r\n der\n"
                ),
                concat!(
                    "Die Hasen klagten einst uber ihre Lage; \"wir leben\", ",
                    "sprach ein Redner, \"in steter Furcht vor Menschen und ",
                    "Tieren, eine Beute der Hunde, der\n"
                ),
            ),
            ("\n\n", "\n\n"),
        ] {
            assert_eq!(
                super::decode_quoted_printable(encoded_str.as_bytes()).unwrap_or_default(),
                expected_result.as_bytes(),
                "Failed for {:?}",
                encoded_str
            );
        }
    }

    #[test]
    fn decode_quoted_printable_mime() {
        for (encoded_str, expected_result) in [
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--boundary",
                "— Antoine de Saint-Exupéry",
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--\n--boundary",
                "— Antoine de Saint-Exupéry\n--",
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry=\n--\n--boundary",
                "— Antoine de Saint-Exupéry--",
            ),
            (
                concat!(
                    "J'interdis aux marchands de vanter trop leurs marchandises. ",
                    "Car ils se font=\nvite p=C3=A9dagogues et t'enseignent comme but ce ",
                    "qui n'est par essence qu=\n'un moyen, et te trompant ainsi sur la route ",
                    "=C3=A0 suivre les voil=C3=\n=A0 bient=C3=B4t qui te d=C3=A9gradent, car ",
                    "si leur musique est vulgaire il=\ns te fabriquent pour te la vendre une ",
                    "=C3=A2me vulgaire.\n=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry, ",
                    "Citadelle (1948)\r\n--boundary--"
                ),
                concat!(
                    "J'interdis aux marchands de vanter trop leurs marchandises. ",
                    "Car ils se fontvite pédagogues et t'enseignent comme but ce qui ",
                    "n'est par essence qu'un moyen, et te trompant ainsi sur la route ",
                    "à suivre les voilà bientôt qui te dégradent, car si leur musique ",
                    "est vulgaire ils te fabriquent pour te la vendre une âme vulgaire.\n",
                    "— Antoine de Saint-Exupéry, Citadelle (1948)"
                ),
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--\n--boundary",
                "— Antoine de Saint-Exupéry\n--",
            ),
            (
                concat!(
                    "Die Hasen klagten einst uber ihre Lage; \"wir ",
                    "leben\", sprach ein=\r\n Redner, \"in steter Furcht vor Menschen",
                    " und Tieren, eine Beute der Hunde,=\r\n der\n\r\n--boundary \n"
                ),
                concat!(
                    "Die Hasen klagten einst uber ihre Lage; \"wir leben\", ",
                    "sprach ein Redner, \"in steter Furcht vor Menschen und ",
                    "Tieren, eine Beute der Hunde, der\n"
                ),
            ),
        ] {
            let mut s = MessageStream::new(encoded_str.as_bytes());
            let (_, result) = super::decode_quoted_printable_mime(&mut s, b"boundary");

            assert_eq!(
                result,
                expected_result.as_bytes(),
                "Failed for {:?}",
                encoded_str
            );
        }
    }

    #[test]
    fn decode_quoted_printable_word() {
        for (encoded_str, expected_result) in [
            ("this=20is=20some=20text?=", "this is some text"),
            ("this=20is=20\n  some=20text?=", "this is some text"),
            ("this is some text?=", "this is some text"),
            ("Keith_Moore?=", "Keith Moore"),
            ("=2=123?=", ""),
            ("= 20?=", ""),
            ("=====?=", ""),
            ("=20=20=XX?=", ""),
            ("=AX?=", ""),
            ("=\n=\n==?=", ""),
            ("=\r=1z?=", ""),
            ("=|?=", ""),
            ("????????=", "???????"),
            ("\n\n", ""),
        ] {
            let (bytes, mut result) = super::decode_quoted_printable_word(encoded_str.as_bytes());
            if bytes == usize::MAX {
                result.clear();
            }

            assert_eq!(
                result,
                expected_result.as_bytes(),
                "Failed for {:?}",
                encoded_str
            );
        }
    }
}

/*
 * Adapted from Daniel Lemire's source:
 * https://github.com/lemire/Code-used-on-Daniel-Lemire-s-blog/blob/master/2019/04/17/hexparse.cpp
 *
 */

pub static HEX_MAP: &[i8] = &[
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 10,
    11, 12, 13, 14, 15, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
