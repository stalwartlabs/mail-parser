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

pub fn decode_base64(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut chunk: u32 = 0;
    let mut byte_count: u8 = 0;

    let mut buf = Vec::with_capacity(bytes.len() / 4 * 3);

    for &ch in bytes {
        #[cfg(feature = "ludicrous_mode")]
        let val = unsafe {
            *BASE64_MAP
                .get_unchecked(byte_count as usize)
                .get_unchecked(ch as usize)
        };
        #[cfg(not(feature = "ludicrous_mode"))]
        let val = BASE64_MAP[byte_count as usize][ch as usize];

        if val < 0x01ffffff {
            byte_count = (byte_count + 1) & 3;

            if byte_count == 1 {
                chunk = val;
            } else {
                chunk |= val;

                if byte_count == 0 {
                    #[cfg(feature = "ludicrous_mode")]
                    unsafe {
                        buf.extend_from_slice(chunk.to_le_bytes().get_unchecked(0..3));
                    }
                    #[cfg(not(feature = "ludicrous_mode"))]
                    buf.extend_from_slice(&chunk.to_le_bytes()[0..3]);
                }
            }
        } else if ch == b'=' {
            match byte_count {
                1 | 2 => {
                    #[cfg(feature = "ludicrous_mode")]
                    unsafe {
                        buf.push(*chunk.to_le_bytes().get_unchecked(0));
                    }
                    #[cfg(not(feature = "ludicrous_mode"))]
                    buf.push(chunk.to_le_bytes()[0]);
                    byte_count = 0;
                }
                3 => {
                    #[cfg(feature = "ludicrous_mode")]
                    unsafe {
                        buf.extend_from_slice(chunk.to_le_bytes().get_unchecked(0..2));
                    }
                    #[cfg(not(feature = "ludicrous_mode"))]
                    buf.extend_from_slice(&chunk.to_le_bytes()[0..2]);
                    byte_count = 0;
                }
                0 => (),
                _ => {
                    return None;
                }
            }
        } else if !ch.is_ascii_whitespace() {
            return None;
        }
    }

    buf.into()
}

//impl<'x> MessageStream<'x> {

pub fn decode_base64_mime<'x>(
    stream: &mut MessageStream<'x>,
    boundary: &[u8],
) -> (usize, Cow<'x, [u8]>) {
    let mut chunk: u32 = 0;
    let mut byte_count: u8 = 0;

    let mut buf = Vec::with_capacity((stream.data.len() - stream.pos) / 4 * 3);
    let mut pos = stream.pos;

    let mut last_ch = b'\n';
    let mut end_pos = stream.pos;
    let mut iter = stream.data[stream.pos..].iter();

    while let Some(&ch) = iter.next() {
        pos += 1;

        #[cfg(feature = "ludicrous_mode")]
        let val = unsafe {
            *BASE64_MAP
                .get_unchecked(byte_count as usize)
                .get_unchecked(ch as usize)
        };
        #[cfg(not(feature = "ludicrous_mode"))]
        let val = BASE64_MAP[byte_count as usize][ch as usize];

        if val < 0x01ffffff {
            byte_count = (byte_count + 1) & 3;

            if byte_count == 1 {
                chunk = val;
            } else {
                chunk |= val;

                if byte_count == 0 {
                    #[cfg(feature = "ludicrous_mode")]
                    unsafe {
                        buf.extend_from_slice(chunk.to_le_bytes().get_unchecked(0..3));
                    }
                    #[cfg(not(feature = "ludicrous_mode"))]
                    buf.extend_from_slice(&chunk.to_le_bytes()[0..3]);
                }
            }
        } else {
            match ch {
                b'=' => match byte_count {
                    1 | 2 => {
                        #[cfg(feature = "ludicrous_mode")]
                        unsafe {
                            buf.push(*chunk.to_le_bytes().get_unchecked(0));
                        }
                        #[cfg(not(feature = "ludicrous_mode"))]
                        buf.push(chunk.to_le_bytes()[0]);
                        byte_count = 0;
                    }
                    3 => {
                        #[cfg(feature = "ludicrous_mode")]
                        unsafe {
                            buf.extend_from_slice(chunk.to_le_bytes().get_unchecked(0..2));
                        }
                        #[cfg(not(feature = "ludicrous_mode"))]
                        buf.extend_from_slice(&chunk.to_le_bytes()[0..2]);
                        byte_count = 0;
                    }
                    0 => (),
                    _ => {
                        return (usize::MAX, b""[..].into());
                    }
                },
                b'\n' => end_pos = if last_ch == b'\r' { pos - 2 } else { pos - 1 },
                b' ' | b'\t' | b'\r' => (),
                b'-' => {
                    return if !boundary.is_empty()
                        && matches!(iter.next(), Some(b'-'))
                        && stream.data.get(pos + 1..pos + 1 + boundary.len()) == Some(boundary)
                        && matches!(
                            stream.data.get(pos + 1 + boundary.len()..),
                            Some([b'\n' | b'\r' | b' ' | b'\t', ..])
                                | Some([b'-', b'-', ..])
                                | Some([])
                                | None
                        ) {
                        stream.pos = pos + boundary.len() + 1;
                        buf.shrink_to_fit();
                        (if last_ch == b'\n' { end_pos } else { pos - 1 }, buf.into())
                    } else {
                        (usize::MAX, b""[..].into())
                    };
                }
                _ => {
                    return (usize::MAX, b""[..].into());
                }
            }
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

pub fn decode_base64_word(bytes: &[u8]) -> (usize, Vec<u8>) {
    let mut chunk: u32 = 0;
    let mut byte_count: u8 = 0;

    let mut bytes_read = 0;

    let mut buf = Vec::with_capacity(64);
    let mut iter = bytes.iter();

    while let Some(&ch) = iter.next() {
        bytes_read += 1;

        match ch {
            b'=' => {
                match byte_count {
                    1 | 2 => {
                        #[cfg(feature = "ludicrous_mode")]
                        unsafe {
                            buf.push(*chunk.to_le_bytes().get_unchecked(0));
                        }
                        #[cfg(not(feature = "ludicrous_mode"))]
                        buf.push(chunk.to_le_bytes()[0]);
                        byte_count = 0;
                    }
                    3 => {
                        #[cfg(feature = "ludicrous_mode")]
                        unsafe {
                            buf.extend_from_slice(chunk.to_le_bytes().get_unchecked(0..2));
                        }
                        #[cfg(not(feature = "ludicrous_mode"))]
                        buf.extend_from_slice(&chunk.to_le_bytes()[0..2]);
                        byte_count = 0;
                    }
                    0 => (),
                    _ => {
                        // Invalid
                        break;
                    }
                }
            }
            b'?' => {
                if let Some(b'=') = iter.next() {
                    return (bytes_read + 1, buf);
                } else {
                    break;
                }
            }
            b'\n' => {
                if let Some(b' ' | b'\t') = iter.next() {
                    bytes_read += 1;
                } else {
                    break;
                }
            }
            b' ' | b'\t' | b'\r' => (),
            _ => {
                #[cfg(feature = "ludicrous_mode")]
                let val = unsafe {
                    *BASE64_MAP
                        .get_unchecked(byte_count as usize)
                        .get_unchecked(ch as usize)
                };
                #[cfg(not(feature = "ludicrous_mode"))]
                let val = BASE64_MAP[byte_count as usize][ch as usize];

                if val < 0x01ffffff {
                    byte_count = (byte_count + 1) & 3;

                    if byte_count == 1 {
                        chunk = val;
                    } else {
                        chunk |= val;

                        if byte_count == 0 {
                            #[cfg(feature = "ludicrous_mode")]
                            unsafe {
                                buf.extend_from_slice(chunk.to_le_bytes().get_unchecked(0..3));
                            }
                            #[cfg(not(feature = "ludicrous_mode"))]
                            buf.extend_from_slice(&chunk.to_le_bytes()[0..3]);
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }

    (usize::MAX, buf)
}
//}

#[cfg(test)]
mod tests {
    use crate::parsers::message::MessageStream;

    #[test]
    fn decode_base64() {
        for (encoded_str, expected_result) in [
            ("VGVzdA==", "Test"),
            ("WWU=", "Ye"),
            ("QQ==", "A"),
            ("cm8=", "ro"),
            (
                "QXJlIHlvdSBhIFNoaW1hbm8gb3IgQ2FtcGFnbm9sbyBwZXJzb24/",
                "Are you a Shimano or Campagnolo person?",
            ),
            (
                "PCFET0NUWVBFIGh0bWw+CjxodG1sPgo8Ym9keT4KPC9ib2R5Pgo8L2h0bWw+Cg==",
                "<!DOCTYPE html>\n<html>\n<body>\n</body>\n</html>\n",
            ),
            (
                "PCFET0NUWVBFIGh0bWw+CjxodG1sPg\no8Ym9ke\nT4KPC 9ib2R5Pg\n o8L2h0bWw+Cg==",
                "<!DOCTYPE html>\n<html>\n<body>\n</body>\n</html>\n",
            ),
            ("w6HDqcOtw7PDug==", "áéíóú"),
            ("====", ""),
            ("w6HDq!cOtw7PDug=", ""),
            ("w6 HD qcOt", "áéí"),
            ("cmáé", ""),
            ("áé", ""),
            ("w\n6\nH\nD\nq\nc\nO\nt\nw\n7\n P\tD u g\n==", "áéíóú"),
            ("w6HDqcOtw7PDug==", "áéíóú"),
        ] {
            assert_eq!(
                super::decode_base64(encoded_str.as_bytes()).unwrap_or_default(),
                expected_result.as_bytes(),
                "Failed for {:?}",
                encoded_str
            );
        }
    }

    #[test]
    fn decode_base64_mime() {
        for (encoded_str, expected_result) in [
            ("VGVzdA==\r\n--boundary\n", "Test"),
            (
                "PCFET0NUWVBFIGh0bWw+CjxodG1sPg\no8Ym9ke\nT4KPC 9ib2R5Pg\n o8L2h0bWw+Cg==\r\n--boundary--\r\n",
                "<!DOCTYPE html>\n<html>\n<body>\n</body>\n</html>\n",
            ),
            ("w6HDqcOtw7PDug==\r\n--boundary \n", "áéíóú"),
            ("w\n6\nH\nD\nq\nc\nO\nt\nw\n7\n P\tD u g\n==\r\n--boundary\n", "áéíóú"),
            ("w6HDqcOtw7PDug==--boundary", "áéíóú"),
            (
                "w6HDqcOtw7PDug==\n--boundary--",
                "áéíóú",
            ),
            (
                "w\n6\nH\nD\nq\nc\nO\nt\nw\n7\n P\tD u g\n==\n--boundary",
                "áéíóú",
            ),
        ] {
            let mut s = MessageStream::new(encoded_str.as_bytes());
            let (_, result) = super::decode_base64_mime(&mut s, b"boundary");

            assert_eq!(
                result,
                expected_result.as_bytes(),
                "Failed for {:?}",
                encoded_str
            );
        }
    }

    #[test]
    fn decode_base64_word() {
        for (encoded_str, expected_result) in [
            ("w 6 H D q c O t w 7 P D u g==  ?=", "áéíóú"),
            ("w6HDqcOtw7PDug==?=", "áéíóú"),
            ("w6HDqc\n  Otw7PDug==?=", "áéíóú"),
            ("w6HDqcOtw7PDug================?=", "áéíóú"),
            ("?=", ""),
        ] {
            let (bytes, result) = super::decode_base64_word(encoded_str.as_bytes());
            assert!(bytes != usize::MAX);
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
 * Table adapted from Nick Galbreath's "High performance base64 encoder / decoder"
 *
 * Copyright 2005, 2006, 2007 Nick Galbreath -- nickg [at] modp [dot] com
 * All rights reserved.
 *
 * http://code.google.com/p/stringencoders/
 *
 * Released under bsd license.
 *
 */

pub static BASE64_MAP: &[&[u32]] = &[
    &[
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x000000f8, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x000000fc, 0x000000d0,
        0x000000d4, 0x000000d8, 0x000000dc, 0x000000e0, 0x000000e4, 0x000000e8, 0x000000ec,
        0x000000f0, 0x000000f4, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x00000000, 0x00000004, 0x00000008, 0x0000000c, 0x00000010,
        0x00000014, 0x00000018, 0x0000001c, 0x00000020, 0x00000024, 0x00000028, 0x0000002c,
        0x00000030, 0x00000034, 0x00000038, 0x0000003c, 0x00000040, 0x00000044, 0x00000048,
        0x0000004c, 0x00000050, 0x00000054, 0x00000058, 0x0000005c, 0x00000060, 0x00000064,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x00000068,
        0x0000006c, 0x00000070, 0x00000074, 0x00000078, 0x0000007c, 0x00000080, 0x00000084,
        0x00000088, 0x0000008c, 0x00000090, 0x00000094, 0x00000098, 0x0000009c, 0x000000a0,
        0x000000a4, 0x000000a8, 0x000000ac, 0x000000b0, 0x000000b4, 0x000000b8, 0x000000bc,
        0x000000c0, 0x000000c4, 0x000000c8, 0x000000cc, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
    ],
    &[
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x0000e003, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x0000f003, 0x00004003,
        0x00005003, 0x00006003, 0x00007003, 0x00008003, 0x00009003, 0x0000a003, 0x0000b003,
        0x0000c003, 0x0000d003, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x00000000, 0x00001000, 0x00002000, 0x00003000, 0x00004000,
        0x00005000, 0x00006000, 0x00007000, 0x00008000, 0x00009000, 0x0000a000, 0x0000b000,
        0x0000c000, 0x0000d000, 0x0000e000, 0x0000f000, 0x00000001, 0x00001001, 0x00002001,
        0x00003001, 0x00004001, 0x00005001, 0x00006001, 0x00007001, 0x00008001, 0x00009001,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x0000a001,
        0x0000b001, 0x0000c001, 0x0000d001, 0x0000e001, 0x0000f001, 0x00000002, 0x00001002,
        0x00002002, 0x00003002, 0x00004002, 0x00005002, 0x00006002, 0x00007002, 0x00008002,
        0x00009002, 0x0000a002, 0x0000b002, 0x0000c002, 0x0000d002, 0x0000e002, 0x0000f002,
        0x00000003, 0x00001003, 0x00002003, 0x00003003, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
    ],
    &[
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x00800f00, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x00c00f00, 0x00000d00,
        0x00400d00, 0x00800d00, 0x00c00d00, 0x00000e00, 0x00400e00, 0x00800e00, 0x00c00e00,
        0x00000f00, 0x00400f00, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x00000000, 0x00400000, 0x00800000, 0x00c00000, 0x00000100,
        0x00400100, 0x00800100, 0x00c00100, 0x00000200, 0x00400200, 0x00800200, 0x00c00200,
        0x00000300, 0x00400300, 0x00800300, 0x00c00300, 0x00000400, 0x00400400, 0x00800400,
        0x00c00400, 0x00000500, 0x00400500, 0x00800500, 0x00c00500, 0x00000600, 0x00400600,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x00800600,
        0x00c00600, 0x00000700, 0x00400700, 0x00800700, 0x00c00700, 0x00000800, 0x00400800,
        0x00800800, 0x00c00800, 0x00000900, 0x00400900, 0x00800900, 0x00c00900, 0x00000a00,
        0x00400a00, 0x00800a00, 0x00c00a00, 0x00000b00, 0x00400b00, 0x00800b00, 0x00c00b00,
        0x00000c00, 0x00400c00, 0x00800c00, 0x00c00c00, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
    ],
    &[
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x003e0000, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x003f0000, 0x00340000,
        0x00350000, 0x00360000, 0x00370000, 0x00380000, 0x00390000, 0x003a0000, 0x003b0000,
        0x003c0000, 0x003d0000, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x00000000, 0x00010000, 0x00020000, 0x00030000, 0x00040000,
        0x00050000, 0x00060000, 0x00070000, 0x00080000, 0x00090000, 0x000a0000, 0x000b0000,
        0x000c0000, 0x000d0000, 0x000e0000, 0x000f0000, 0x00100000, 0x00110000, 0x00120000,
        0x00130000, 0x00140000, 0x00150000, 0x00160000, 0x00170000, 0x00180000, 0x00190000,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x001a0000,
        0x001b0000, 0x001c0000, 0x001d0000, 0x001e0000, 0x001f0000, 0x00200000, 0x00210000,
        0x00220000, 0x00230000, 0x00240000, 0x00250000, 0x00260000, 0x00270000, 0x00280000,
        0x00290000, 0x002a0000, 0x002b0000, 0x002c0000, 0x002d0000, 0x002e0000, 0x002f0000,
        0x00300000, 0x00310000, 0x00320000, 0x00330000, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
        0x01ffffff, 0x01ffffff, 0x01ffffff, 0x01ffffff,
    ],
];
