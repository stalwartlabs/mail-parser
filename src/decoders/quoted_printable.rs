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

use crate::parsers::message_stream::MessageStream;

#[derive(PartialEq, Debug)]
enum QuotedPrintableState {
    None,
    Eq,
    Hex1,
}

pub trait QuotedPrintableDecoder<'x> {
    fn decode_quoted_printable(
        &self,
        boundary: &[u8],
        is_word: bool,
    ) -> (bool, bool, Option<&'x [u8]>);
}

// Miri does not play well with UnsafeCells, see https://github.com/rust-lang/miri/issues/1665
//
// When testing under Miri,
//     (*data)[i] = val;
// is used instead of
//     *(*data).get_unchecked_mut(i) = val;
//

impl<'x> QuotedPrintableDecoder<'x> for MessageStream<'x> {
    fn decode_quoted_printable(
        &self,
        boundary: &[u8],
        is_word: bool,
    ) -> (bool, bool, Option<&'x [u8]>) {
        unsafe {
            let mut success = boundary.is_empty();
            let mut is_utf8_safe = true;

            let mut data = &mut *self.data.get();
            let data_len = (*data).len();

            let stream_pos = &mut *self.pos.get();
            let start_pos = *stream_pos;
            let mut read_pos = *stream_pos;
            let mut write_pos = *stream_pos;

            let mut state = QuotedPrintableState::None;
            let mut match_count = 0;
            let mut hex1 = 0;

            while read_pos < data_len {
                let ch = *(*data).get_unchecked(read_pos);
                read_pos += 1;

                if !success {
                    if ch == *boundary.get_unchecked(match_count) {
                        match_count += 1;
                        if match_count == boundary.len() {
                            let done = is_word || self.is_boundary_end(read_pos);
                            data = &mut *self.data.get(); // Avoid violating the Stacked Borrows rules

                            if done {
                                success = true;
                                break;
                            }
                        } else {
                            continue;
                        }
                    }

                    if match_count > 0 {
                        for ch in boundary[..match_count].iter() {
                            if *ch != b'\n' || QuotedPrintableState::Eq != state {
                                #[cfg(miri)]
                                {
                                    (*data)[write_pos] = *ch;
                                }
                                #[cfg(not(miri))]
                                {
                                    *((*data).get_unchecked_mut(write_pos)) = *ch;
                                }
                                write_pos += 1;
                                if is_utf8_safe && *ch > 0x7f {
                                    is_utf8_safe = false;
                                }
                            }
                        }
                        state = QuotedPrintableState::None;

                        if ch == *boundary.get_unchecked(0) {
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
                            success = false;
                            break;
                        }
                    }
                    b'\n' => {
                        if is_word {
                            success = false;
                            break;
                        } else if QuotedPrintableState::Eq == state {
                            state = QuotedPrintableState::None;
                        } else {
                            #[cfg(miri)]
                            {
                                (*data)[write_pos] = b'\n';
                            }
                            #[cfg(not(miri))]
                            {
                                *((*data).get_unchecked_mut(write_pos)) = b'\n';
                            }

                            write_pos += 1;
                        }
                    }
                    b'_' if is_word => {
                        #[cfg(miri)]
                        {
                            (*data)[write_pos] = b' ';
                        }
                        #[cfg(not(miri))]
                        {
                            *((*data).get_unchecked_mut(write_pos)) = b' ';
                        }
                        write_pos += 1;
                    }
                    b'\r' => (),
                    _ => match state {
                        QuotedPrintableState::None => {
                            #[cfg(miri)]
                            {
                                (*data)[write_pos] = ch;
                            }
                            #[cfg(not(miri))]
                            {
                                *((*data).get_unchecked_mut(write_pos)) = ch;
                            }
                            write_pos += 1;
                            if is_utf8_safe && ch > 0x7f {
                                is_utf8_safe = false;
                            }
                        }
                        QuotedPrintableState::Eq => {
                            hex1 = *HEX_MAP.get_unchecked(ch as usize);
                            if hex1 != -1 {
                                state = QuotedPrintableState::Hex1;
                            } else {
                                success = false;
                                break;
                            }
                        }
                        QuotedPrintableState::Hex1 => {
                            let hex2 = *HEX_MAP.get_unchecked(ch as usize);

                            state = QuotedPrintableState::None;
                            if hex2 != -1 {
                                let ch = ((hex1 as u8) << 4) | hex2 as u8;

                                #[cfg(miri)]
                                {
                                    (*data)[write_pos] = ch;
                                }
                                #[cfg(not(miri))]
                                {
                                    *((*data).get_unchecked_mut(write_pos)) = ch;
                                }

                                write_pos += 1;
                                if is_utf8_safe && ch > 0x7f {
                                    is_utf8_safe = false;
                                }
                            } else {
                                success = false;
                                break;
                            }
                        }
                    },
                }
            }

            *stream_pos = read_pos;

            (
                success,
                is_utf8_safe,
                if write_pos > start_pos {
                    Some((*data).get_unchecked(start_pos..write_pos))
                } else {
                    None
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        decoders::quoted_printable::QuotedPrintableDecoder, parsers::message_stream::MessageStream,
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
                "",
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
            let mut str = input.0.to_string();
            let stream = MessageStream::new(unsafe { str.as_bytes_mut() });

            let (success, _, result) = stream.decode_quoted_printable(input.2.as_bytes(), input.3);

            assert_eq!(success, !input.1.is_empty(), "Failed for '{:?}'", input.0);

            if !input.1.is_empty() {
                let result_str = std::str::from_utf8(result.unwrap()).unwrap();

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
