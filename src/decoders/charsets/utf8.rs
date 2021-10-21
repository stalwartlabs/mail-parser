use std::{cell::UnsafeCell, char::REPLACEMENT_CHARACTER};

use crate::decoders::{Writer, buffer_writer::BufferWriter};

enum Utf8State {
    Start,
    Shift12,
    Shift6,
    Shift0,
}

pub struct Utf8Decoder<'x> {
    state: UnsafeCell<Utf8State>,
    char: UnsafeCell<u32>,
    buf: &'x BufferWriter,
}

impl<'x> Writer for Utf8Decoder<'x> {
    fn write_byte(&self, byte: &u8) -> bool {
        unsafe {
            let state = &mut *self.state.get();
            let char = &mut *self.char.get();

            match *state {
                Utf8State::Start => {
                    if *byte < 0x80 {
                        self.buf.write_byte(byte);
                    } else if (*byte & 0xe0) == 0xc0 {
                        *char = (*byte as u32 & 0x1f) << 6;
                        *state = Utf8State::Shift0;
                    } else if (*byte & 0xf0) == 0xe0 {
                        *char = (*byte as u32 & 0x0f) << 12;
                        *state = Utf8State::Shift6;
                    } else if (*byte & 0xf8) == 0xf0 && (*byte <= 0xf4) {
                        *char = (*byte as u32 & 0x07) << 18;
                        *state = Utf8State::Shift12;
                    } else {
                        self.buf.write_bytes("�".as_bytes());
                    }
                }
                Utf8State::Shift12 => {
                    *char |= (*byte as u32 & 0x3f) << 12;
                    *state = Utf8State::Shift6;
                }
                Utf8State::Shift6 => {
                    *char |= (*byte as u32 & 0x3f) << 6;
                    *state = Utf8State::Shift0;
                }
                Utf8State::Shift0 => {
                    *char |= *byte as u32 & 0x3f;
                    *state = Utf8State::Start;
                    self.buf
                        .write_bytes(char::from_u32(*char).unwrap_or(REPLACEMENT_CHARACTER).to_string().as_bytes());
                    *char = 0;
                }
            }
        }

        true
    }
}

impl Utf8Decoder<'_> {
    pub fn new<'x>(buf: &'x BufferWriter) -> Utf8Decoder<'x> {
        Utf8Decoder {
            buf,
            state: Utf8State::Start.into(),
            char: 0.into(),
        }
    }

    pub fn get_utf8<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(Utf8Decoder::new(buf))
    }
}

#[cfg(test)]
mod tests {
    use crate::decoders::{Writer, buffer_writer::BufferWriter};

    use super::Utf8Decoder;

    #[test]
    fn decode_utf8() {
        let inputs = [
            (b"Lorem ipsum".to_vec(), "Lorem ipsum"),
            (b"Th\xc3\xads \xc3\xads v\xc3\xa1l\xc3\xadd \xc3\x9aTF8".to_vec(), "Thís ís válíd ÚTF8"),
            (b"\xe3\x83\x8f\xe3\x83\xad\xe3\x83\xbc\xe3\x83\xbb\xe3\x83\xaf\xe3\x83\xbc\xe3\x83\xab\xe3\x83\x89".to_vec(), "ハロー・ワールド"),
            (b"\xec\x95\x88\xeb\x85\x95\xed\x95\x98\xec\x84\xb8\xec\x9a\x94 \xec\x84\xb8\xea\xb3\x84".to_vec(), "안녕하세요 세계"),
            (b"love: \xe2\x9d\xa4\xef\xb8\x8f".to_vec(), "love: ❤️"),
            (b"\xec \x95\x88 \xeb\x85\x95 \xed\x95\x98\xec\x84\xb8 \xec\x9a\x94 \xec\x84\xb8\xea\xb3 \x84".to_vec(), "정� 녕 하세 요 세고�"),
        ];

        for input in inputs {
            let buffer = BufferWriter::with_capacity(input.0.len() * 2);
            let parser = Utf8Decoder::new(&buffer);
            parser.write_bytes(&input.0);

            assert_eq!(buffer.get_string().unwrap(), input.1);
        }
    }
}
