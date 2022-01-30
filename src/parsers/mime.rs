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

use crate::decoders::DecodeResult;

use super::message::MessageStream;

pub fn seek_next_part(stream: &mut MessageStream, boundary: &[u8]) -> bool {
    if !boundary.is_empty() {
        let mut pos = stream.pos;

        let mut match_count = 0;

        for ch in &stream.data[pos..] {
            pos += 1;

            if ch == &boundary[match_count] {
                match_count += 1;
                if match_count == boundary.len() {
                    stream.pos = pos;
                    return true;
                } else {
                    continue;
                }
            } else if match_count > 0 {
                if ch == &boundary[0] {
                    match_count = 1;
                    continue;
                } else {
                    match_count = 0;
                }
            }
        }
    }

    false
}

pub fn get_bytes_to_boundary<'x>(
    stream: &MessageStream<'x>,
    start_pos: usize,
    boundary: &[u8],
    _is_word: bool,
) -> (usize, DecodeResult) {
    let mut read_pos = start_pos;

    if !boundary.is_empty() {
        let mut match_count = 0;

        for ch in &stream.data[read_pos..] {
            read_pos += 1;

            if ch == &boundary[match_count] {
                match_count += 1;
                if match_count == boundary.len() {
                    if is_boundary_end(stream, read_pos) {
                        let mut match_pos = read_pos - match_count;

                        if let Some(b'\r') = stream.data.get(match_pos - 1) {
                            match_pos -= 1;
                        }

                        return (
                            read_pos - start_pos,
                            if start_pos < match_pos {
                                DecodeResult::Borrowed((start_pos, match_pos))
                            } else {
                                DecodeResult::Empty
                            },
                        );
                    } else {
                        match_count = 0;
                    }
                }
                continue;
            } else if match_count > 0 {
                if ch == &boundary[0] {
                    match_count = 1;
                    continue;
                } else {
                    match_count = 0;
                }
            }
        }

        (0, DecodeResult::Empty)
    } else if start_pos < stream.data.len() {
        (
            stream.data.len() - start_pos,
            DecodeResult::Borrowed((start_pos, stream.data.len())),
        )
    } else {
        (0, DecodeResult::Empty)
    }
}

#[inline(always)]
pub fn is_boundary_end(stream: &MessageStream, pos: usize) -> bool {
    matches!(
        stream.data.get(pos..),
        Some([b'\n' | b'\r' | b' ' | b'\t', ..]) | Some([b'-', b'-', ..]) | Some([]) | None
    )
}

pub fn skip_multipart_end(stream: &mut MessageStream) -> bool {
    match stream.data.get(stream.pos..stream.pos + 2) {
        Some(b"--") => {
            if let Some(byte) = stream.data.get(stream.pos + 2) {
                if !(*byte).is_ascii_whitespace() {
                    return false;
                }
            }
            stream.pos += 2;
            true
        }
        _ => false,
    }
}

#[inline(always)]
pub fn skip_crlf(stream: &mut MessageStream) {
    for ch in &stream.data[stream.pos..] {
        match ch {
            b'\r' | b' ' | b'\t' => stream.pos += 1,
            b'\n' => {
                stream.pos += 1;
                break;
            }
            _ => break,
        }
    }
}

#[inline(always)]
pub fn seek_crlf_end(stream: &MessageStream, mut start_pos: usize) -> usize {
    for ch in &stream.data[start_pos..] {
        if ch.is_ascii_whitespace() {
            start_pos += 1;
        } else {
            break;
        }
    }
    start_pos
}
