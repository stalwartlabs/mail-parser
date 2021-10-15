use super::{quoted_printable::HEX_MAP, Writer};

#[derive(PartialEq, Debug)]
enum HexState {
    None,
    Percent,
    Hex1,
}

pub fn decode_hex(src: &[u8], dest: &mut dyn Writer) -> bool {
    let mut state = HexState::None;
    let mut hex1 = 0;

    for ch in src {
        match ch {
            b'%' => {
                if let HexState::None = state {
                    state = HexState::Percent
                } else {
                    return false;
                }
            }
            _ => match state {
                HexState::None => {
                    dest.write_byte(ch);
                }
                HexState::Percent => {
                    hex1 = unsafe { *HEX_MAP.get_unchecked(*ch as usize) };
                    if hex1 != -1 {
                        state = HexState::Hex1;
                    } else {
                        return false;
                    }
                }
                HexState::Hex1 => {
                    let hex2 = unsafe { *HEX_MAP.get_unchecked(*ch as usize) };

                    state = HexState::None;
                    if hex2 != -1 {
                        dest.write_byte(&(((hex1 as u8) << 4) | hex2 as u8));
                    } else {
                        return false;
                    }
                }
            },
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::decoders::{Writer, buffer_writer::BufferWriter, hex::decode_hex};

    #[test]
    fn decode_hex_line() {
        let inputs = [
            ("this%20is%20some%20text", "this is some text"),
            ("this is some text", "this is some text"),
        ];

        for input in inputs {
            let mut writer = BufferWriter::with_capacity(input.0.len());

            assert!(
                decode_hex(input.0.as_bytes(), &mut writer),
                "Failed for '{}'",
                input.0.escape_debug()
            );

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
