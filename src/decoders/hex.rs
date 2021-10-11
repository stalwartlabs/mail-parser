use super::{quoted_printable::HEX_MAP, Decoder, DecoderResult};

enum HexState {
    None,
    Eq,
    Hex1,
}

pub struct HexDecoder {
    state: HexState,
    hex_1: i8,
}

impl HexDecoder {
    pub fn new() -> HexDecoder {
        HexDecoder {
            state: HexState::None,
            hex_1: 0,
        }
    }
}

impl Decoder for HexDecoder {
    fn ingest(&mut self, ch: &u8) -> DecoderResult {
        match self.state {
            HexState::None => match ch {
                b'%' => {
                    self.state = HexState::Eq;
                }
                0..=126 => {
                    return DecoderResult::Byte(*ch);
                }
                _ => {
                    return DecoderResult::Byte(b'!');
                }
            },
            HexState::Eq => {
                if *ch != b'\n' && *ch != b'\r' {
                    self.hex_1 = unsafe { *HEX_MAP.get_unchecked(*ch as usize) };
                    if self.hex_1 != -1 {
                        self.state = HexState::Hex1;
                    } else {
                        self.state = HexState::None;
                        return DecoderResult::Error;
                    }
                } else {
                    self.state = HexState::None;
                }
            }
            HexState::Hex1 => {
                let hex2 = unsafe { *HEX_MAP.get_unchecked(*ch as usize) };

                self.state = HexState::None;
                if hex2 == -1 {
                    return DecoderResult::Error;
                } else {
                    return DecoderResult::Byte(((self.hex_1 as u8) << 4) | hex2 as u8);
                }
            }
        }

        DecoderResult::NeedData
    }
}

#[cfg(test)]
mod tests {
    use crate::decoders::{hex::HexDecoder, Decoder, DecoderResult};

    #[test]
    fn hex_decode_line() {
        let inputs = [
            ("this%20is%20some%20text".as_bytes(), "this is some text"),
            ("this is some text".as_bytes(), "this is some text"),
        ];

        for input in inputs {
            let mut decoder = HexDecoder::new();
            let mut result = String::new();

            for ch in input.0 {
                match decoder.ingest(ch) {
                    DecoderResult::Byte(val) => {
                        result.push(char::from(val));
                    }
                    DecoderResult::NeedData => (),
                    _ => {
                        panic!("Error decoding '{}'", std::str::from_utf8(input.0).unwrap());
                    }
                }
            }

            assert_eq!(result, input.1);
        }
    }
}
