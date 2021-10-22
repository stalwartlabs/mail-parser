use crate::parsers::message_stream::MessageStream;

use super::decoder::Decoder;

pub trait BytesDecoder<'x> {
    fn decode_bytes(&self, boundary: &[u8], _is_word: bool, dest: &mut dyn Decoder) -> bool;
    fn get_raw_bytes(&self, boundary: &[u8]) -> (bool, bool, Option<&'x [u8]>);
}

impl<'x> BytesDecoder<'x> for MessageStream<'x> {
    fn decode_bytes(&self, boundary: &[u8], _is_word: bool, dest: &mut dyn Decoder) -> bool {
        let mut pos = self.get_pos();
        let mut match_count = 0;

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
                        dest.write_byte(ch);
                    }

                    if ch == unsafe { boundary.get_unchecked(0) } {
                        match_count = 1;
                        continue;
                    } else {
                        match_count = 0;
                    }
                }
            }

            dest.write_byte(ch);
        }

        if boundary.is_empty() {
            self.set_pos(pos);
            true
        } else {
            false
        }
    }

    fn get_raw_bytes(&self, boundary: &[u8]) -> (bool, bool, Option<&'x [u8]>) {
        let start_pos = self.get_pos();

        return if !boundary.is_empty() {
            let mut pos = start_pos;
            let mut match_count = 0;
            let mut is_utf8_safe = true;

            for ch in self.data[pos..].iter() {
                pos += 1;

                if is_utf8_safe && *ch > 0x7f {
                    is_utf8_safe = false;
                }

                if ch == unsafe { boundary.get_unchecked(match_count) } {
                    match_count += 1;
                    if match_count == boundary.len() {
                        let match_pos = pos - match_count;
                        self.set_pos(pos);
                        return (
                            true,
                            is_utf8_safe,
                            if start_pos < match_pos {
                                self.data.get(start_pos..match_pos)
                            } else {
                                None
                            },
                        );
                    } else {
                        continue;
                    }
                } else if match_count > 0 {
                    if ch == unsafe { boundary.get_unchecked(0) } {
                        match_count = 1;
                        continue;
                    } else {
                        match_count = 0;
                    }
                }
            }

            (false, false, None)
        } else if start_pos < self.data.len() {
            self.set_pos(self.data.len());
            (true, false, self.data.get(start_pos..))
        } else {
            (false, false, None)
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        decoders::{
            buffer_writer::BufferWriter,
            bytes::BytesDecoder,
            decoder::{Decoder, RawDecoder},
        },
        parsers::message_stream::MessageStream,
    };

    #[test]
    fn decode_bytes_input() {
        let inputs = [
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--\n--boundary",
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry\n--",
                "\n--boundary",
                false,
            ),
            (
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry=\n--boundary",
                "=E2=80=94=E2=80=89Antoine de Saint-Exup=C3=A9ry=",
                "\n--boundary",
                false,
            ),
            ("this is some text", "this is some text", "", true),
        ];

        for input in inputs {
            let stream = MessageStream::new(input.0.as_bytes());
            let mut buffer = BufferWriter::alloc_buffer(input.0.len() * 3);
            let len = {
                let mut decoder = RawDecoder::new(&mut buffer);

                assert!(
                    stream.decode_bytes(input.2.as_bytes(), false, &mut decoder),
                    "Failed for '{:?}'",
                    input.0
                );
                decoder.len()
            };

            if !input.1.is_empty() {
                let result_str = std::str::from_utf8(&buffer[..len]).unwrap();

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
