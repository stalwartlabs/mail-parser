use crate::parsers::message_stream::MessageStream;

use super::Writer;

pub trait BinaryDecoder<'y> {
    fn decode_binary(&self, boundary: &[u8], dest: &mut dyn Writer) -> bool;
}

impl<'x> BinaryDecoder<'x> for MessageStream<'x> {
    fn decode_binary(&self, boundary: &[u8], dest: &mut dyn Writer) -> bool {
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
}

#[cfg(test)]
mod tests {
    use crate::{
        decoders::{binary::BinaryDecoder, buffer_writer::BufferWriter, Writer},
        parsers::message_stream::MessageStream,
    };

    #[test]
    fn decode_binary_input() {
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
            let mut writer = BufferWriter::with_capacity(input.0.len());

            assert!(
                stream.decode_binary(input.2.as_bytes(), &mut writer),
                "Failed for '{:?}'",
                input.0
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
