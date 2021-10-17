use crate::parsers::message_stream::MessageStream;

use super::Writer;

#[derive(PartialEq, Debug)]
enum QuotedPrintableState {
    None,
    Eq,
    Hex1,
}

pub trait QuotedPrintableDecoder<'y> {
    fn decode_quoted_printable(
        &self,
        boundary: &[u8],
        is_word: bool,
        dest: &mut dyn Writer,
    ) -> bool;
}

impl<'x> QuotedPrintableDecoder<'x> for MessageStream<'x> {
    fn decode_quoted_printable(
        &self,
        boundary: &[u8],
        is_word: bool,
        dest: &mut dyn Writer,
    ) -> bool {
        let mut pos = self.get_pos();
        let mut state = QuotedPrintableState::None;
        let mut match_count = 0;
        let mut hex1 = 0;

        for ch in self.data[pos..].iter() {
            pos += 1;

            if match_count < boundary.len() {
                if ch == unsafe { boundary.get_unchecked(match_count) } {
                    match_count += 1;
                    if match_count == boundary.len() {
                        self.set_pos(pos);
                        return true;
                    } else {
                        continue;
                    }
                } else if match_count > 0 {
                    for ch in boundary[..match_count].iter() {
                        if *ch != b'\n' || QuotedPrintableState::Eq != state {
                            dest.write_byte(ch);
                        }
                    }
                    state = QuotedPrintableState::None;

                    if ch == unsafe { boundary.get_unchecked(0) } {
                        match_count = 1;
                        continue;
                    } else {
                        match_count = 0;
                    }
                }
            }

            match ch {
                b'=' => {
                    if let QuotedPrintableState::None = state {
                        state = QuotedPrintableState::Eq
                    } else {
                        return false;
                    }
                }
                b'\n' => {
                    if is_word {
                        return false;
                    } else if QuotedPrintableState::Eq == state {
                        state = QuotedPrintableState::None;
                    } else {
                        dest.write_byte(&b'\n');
                    }
                }
                b'_' if is_word => {
                    dest.write_byte(&b' ');
                }
                _ => match state {
                    QuotedPrintableState::None => {
                        dest.write_byte(ch);
                    }
                    QuotedPrintableState::Eq => {
                        hex1 = unsafe { *HEX_MAP.get_unchecked(*ch as usize) };
                        if hex1 != -1 {
                            state = QuotedPrintableState::Hex1;
                        } else {
                            return false;
                        }
                    }
                    QuotedPrintableState::Hex1 => {
                        let hex2 = unsafe { *HEX_MAP.get_unchecked(*ch as usize) };

                        state = QuotedPrintableState::None;
                        if hex2 != -1 {
                            dest.write_byte(&(((hex1 as u8) << 4) | hex2 as u8));
                        } else {
                            return false;
                        }
                    }
                },
            }
        }

        boundary.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        decoders::{buffer_writer::BufferWriter, quoted_printable::QuotedPrintableDecoder, Writer},
        parsers::message_stream::MessageStream,
    };

    #[test]
    fn decode_quoted_printable() {
        let inputs = [
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
                "",
                false,
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--boundary",
                "— Antoine de Saint-Exupéry",
                "\n--boundary",
                false,
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--\n--boundary",
                "— Antoine de Saint-Exupéry\n--",
                "\n--boundary",
                false,
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry=\n--\n--boundary",
                "— Antoine de Saint-Exupéry--",
                "\n--boundary",
                false,
            ),
            ("this=20is=20some=20text?=", "this is some text", "?=", true),
            ("this is some text?=", "this is some text", "?=", true),
            ("Keith_Moore?=", "Keith Moore", "?=", true),
            ("=2=123?=", "", "?=", true),
            ("= 20?=", "", "?=", true),
            ("=====?=", "", "?=", true),
            ("=20=20=XX?=", "", "?=", true),
            ("=AX?=", "", "?=", true),
            ("=\n=\n==?=", "", "?=", true),
            ("=\r=1z?=", "", "?=", true),
            ("=|?=", "", "?=", true),
            ("????????=", "???????", "?=", true),
        ];

        for input in inputs {
            let stream = MessageStream::new(input.0.as_bytes());
            let mut writer = BufferWriter::with_capacity(input.0.len());

            assert_eq!(
                stream.decode_quoted_printable(input.2.as_bytes(), input.3, &mut writer),
                !input.1.is_empty(),
                "Failed for '{}'",
                input.0.escape_debug()
            );

            if !input.1.is_empty() {
                let result = &writer.get_bytes().unwrap();
                let result_str = std::str::from_utf8(result).unwrap();

                /*println!(
                    "Decoded '{}'\n -> to ->\n'{}'\n{}",
                    input.0.escape_debug(),
                    result_str.escape_debug(),
                    "-".repeat(50)
                );*/

                assert_eq!(
                    input.1,
                    result_str,
                    "Failed for '{}'",
                    input.0.escape_debug()
                );
            }
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
