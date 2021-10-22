use std::char::REPLACEMENT_CHARACTER;

use crate::decoders::decoder::Decoder;

enum Utf8State {
    Start,
    Shift12,
    Shift6,
    Shift0,
}

pub struct Utf8Decoder<'x> {
    state: Utf8State,
    char: u32,
    pos: usize,
    buf: &'x mut [u8],
}

impl<'x> Decoder for Utf8Decoder<'x> {
    fn write_byte(&mut self, byte: &u8) -> bool {
        match self.state {
            Utf8State::Start => {
                if *byte < 0x80 {
                    if let Some(b) = self.buf.get_mut(self.pos) {
                        *b = *byte;
                        self.pos += 1;
                    } else {
                        return false;
                    }
                } else if (*byte & 0xe0) == 0xc0 {
                    self.char = (*byte as u32 & 0x1f) << 6;
                    self.state = Utf8State::Shift0;
                } else if (*byte & 0xf0) == 0xe0 {
                    self.char = (*byte as u32 & 0x0f) << 12;
                    self.state = Utf8State::Shift6;
                } else if (*byte & 0xf8) == 0xf0 && (*byte <= 0xf4) {
                    self.char = (*byte as u32 & 0x07) << 18;
                    self.state = Utf8State::Shift12;
                } else {
                    let bytes = "�".as_bytes();
                    if let Some(b) = self.buf.get_mut(self.pos..self.pos + bytes.len()) {
                        b.copy_from_slice(bytes);
                        self.pos += bytes.len();
                    } else {
                        return false;
                    }
                }
            }
            Utf8State::Shift12 => {
                self.char |= (*byte as u32 & 0x3f) << 12;
                self.state = Utf8State::Shift6;
            }
            Utf8State::Shift6 => {
                self.char |= (*byte as u32 & 0x3f) << 6;
                self.state = Utf8State::Shift0;
            }
            Utf8State::Shift0 => {
                self.char |= *byte as u32 & 0x3f;
                self.state = Utf8State::Start;

                let str = char::from_u32(self.char)
                    .unwrap_or(REPLACEMENT_CHARACTER)
                    .to_string();
                let bytes = str.as_bytes();
                if let Some(b) = self.buf.get_mut(self.pos..self.pos + bytes.len()) {
                    b.copy_from_slice(bytes);
                    self.pos += bytes.len();
                } else {
                    return false;
                }
                self.char = 0;
            }
        }

        true
    }

    fn len(&self) -> usize {
        self.pos
    }

    fn is_utf8_safe(&self) -> bool {
        true
    }
}

impl Utf8Decoder<'_> {
    pub fn new<'x>(buf: &'x mut [u8]) -> Utf8Decoder<'x> {
        Utf8Decoder {
            buf,
            state: Utf8State::Start,
            char: 0,
            pos: 0,
        }
    }

    pub fn get_utf8<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(Utf8Decoder::new(buf))
    }
}

#[cfg(test)]
mod tests {
    use crate::decoders::{buffer_writer::BufferWriter, decoder::Decoder};

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
            let mut buffer = BufferWriter::alloc_buffer(input.1.len() * 3);
            let len = {
                let mut decoder = Utf8Decoder::new(buffer.as_mut());
                decoder.write_bytes(&input.0);
                decoder.len()
            };

            assert_eq!(std::str::from_utf8(&buffer[..len]).unwrap(), input.1);
        }
    }
}
