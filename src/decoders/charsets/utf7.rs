use std::{
    borrow::Cow,
    char::{decode_utf16, REPLACEMENT_CHARACTER},
};

use crate::decoders::base64::{Base64Chunk, BASE64_MAP};

struct Utf7DecoderState {
    utf16_bytes: Vec<u16>,
    pending_byte: Option<u8>,
    b64_bytes: Base64Chunk,
}

fn add_utf16_bytes(state: &mut Utf7DecoderState, n_bytes: usize) {
    unsafe {
        for byte in state.b64_bytes.bytes.get_unchecked(0..n_bytes) {
            if let Some(pending_byte) = state.pending_byte {
                state
                    .utf16_bytes
                    .push(u16::from_be_bytes([pending_byte, *byte]));
                state.pending_byte = None;
            } else {
                state.pending_byte = Some(*byte);
            }
        }
    }
}

pub fn decoder_utf7(bytes: &[u8]) -> Cow<str> {
    let mut result = String::with_capacity(bytes.len());
    let mut byte_count: u8 = 0;
    let mut in_b64 = false;

    let mut state = Utf7DecoderState {
        utf16_bytes: Vec::with_capacity(10),
        pending_byte: None,
        b64_bytes: Base64Chunk { val: 0 },
    };

    for byte in bytes {
        if in_b64 {
            let val = unsafe {
                BASE64_MAP
                    .get_unchecked(byte_count as usize)
                    .get_unchecked(*byte as usize)
            };

            if *val < 0x01ffffff {
                byte_count = (byte_count + 1) & 3;

                if byte_count == 1 {
                    state.b64_bytes.val = *val;
                } else {
                    unsafe {
                        state.b64_bytes.val |= *val;
                    }

                    if byte_count == 0 {
                        add_utf16_bytes(&mut state, 3);
                    }
                }
            } else {
                match byte_count {
                    1 | 2 => {
                        add_utf16_bytes(&mut state, 1);
                    }
                    3 => {
                        add_utf16_bytes(&mut state, 2);
                    }
                    _ => (),
                }

                if !state.utf16_bytes.is_empty() {
                    result.push_str(
                        decode_utf16(state.utf16_bytes.drain(..))
                            .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
                            .collect::<String>()
                            .as_str(),
                    );
                } else if byte_count > 0 || state.pending_byte.is_some() {
                    result.push(REPLACEMENT_CHARACTER);
                } else {
                    result.push('+');
                    result.push(char::from(*byte));
                }

                state.pending_byte = None;
                byte_count = 0;
                in_b64 = false;
            }
        } else if byte == &b'+' {
            in_b64 = true;
        } else {
            result.push(char::from(*byte));
        }
    }

    result.shrink_to_fit();
    result.into()
}

mod tests {
    use crate::decoders::charsets::utf7::decoder_utf7;

    #[test]
    fn decode_utf7() {
        let inputs = [
            ("Hello, World+ACE-", "Hello, World!"),
            ("Hi Mom -+Jjo--!", "Hi Mom -☺-!"),
            ("+ZeVnLIqe-", "日本語"),
            ("Item 3 is +AKM-1.", "Item 3 is £1."),
            ("Plus minus +- -+ +--", "Plus minus +- -+ +--"),
            (
                "+APw-ber ihre mi+AN8-liche Lage+ADs- +ACI-wir",
                "über ihre mißliche Lage; \"wir",
            ),
            (
                concat!(
                    "+ACI-The sayings of Confucius,+ACI- James R. Ware, trans.  +U/BTFw-:\n",
                    "+ZYeB9FH6ckh5Pg-, 1980.\n",
                    "+Vttm+E6UfZM-, +W4tRQ066bOg-, +UxdOrA-:  +Ti1XC2b4Xpc-, 1990."
                ),
                concat!(
                    "\"The sayings of Confucius,\" James R. Ware, trans.  台北:\n",
                    "文致出版社, 1980.\n",
                    "四書五經, 宋元人注, 北京:  中國書店, 1990."
                ),
            ),
        ];

        for input in inputs {
            assert_eq!(decoder_utf7(input.0.as_bytes()), input.1);
        }
    }
}
