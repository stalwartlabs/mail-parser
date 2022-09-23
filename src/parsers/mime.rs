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

use super::message::MessageStream;

pub fn seek_next_part(stream: &mut MessageStream, boundary: &[u8]) -> bool {
    if !boundary.is_empty() {
        let mut pos = stream.pos;

        let mut iter = stream.data[stream.pos..].iter().peekable();
        while let Some(&ch) = iter.next() {
            pos += 1;

            if ch == b'-'
                && matches!(iter.peek(), Some(b'-'))
                && stream.data.get(pos + 1..pos + 1 + boundary.len()) == Some(boundary)
            {
                stream.pos = pos + boundary.len() + 1;
                return true;
            }
        }
    }

    false
}

pub fn get_mime_part<'x>(
    stream: &mut MessageStream<'x>,
    boundary: &[u8],
) -> (usize, Cow<'x, [u8]>) {
    let mut last_ch = b'\n';
    let start_pos = stream.pos;
    let mut end_pos = stream.pos;
    let mut pos = stream.pos;

    let mut iter = stream.data[stream.pos..].iter().peekable();
    while let Some(&ch) = iter.next() {
        pos += 1;

        if ch == b'\n' {
            end_pos = if last_ch == b'\r' { pos - 2 } else { pos - 1 };
        } else if ch == b'-'
            && !boundary.is_empty()
            && matches!(iter.peek(), Some(b'-'))
            && stream.data.get(pos + 1..pos + 1 + boundary.len()) == Some(boundary)
            && matches!(
                stream.data.get(pos + 1 + boundary.len()..),
                Some([b'\n' | b'\r' | b' ' | b'\t', ..]) | Some([b'-', b'-', ..]) | Some([]) | None
            )
        {
            stream.pos = pos + boundary.len() + 1;
            if last_ch != b'\n' {
                end_pos = pos - 1;
            }
            return (end_pos, stream.data[start_pos..end_pos].into());
        }

        last_ch = ch;
    }

    (
        if boundary.is_empty() {
            stream.pos = pos;
            stream.pos
        } else {
            usize::MAX
        },
        stream.data[start_pos..pos].into(),
    )
}

pub fn seek_part_end<'x>(stream: &mut MessageStream<'x>, boundary: Option<&[u8]>) -> (usize, bool) {
    let mut last_ch = b'\n';
    let mut end_pos = stream.pos;

    if let Some(boundary) = boundary {
        let mut iter = stream.data[stream.pos..].iter().peekable();
        while let Some(&ch) = iter.next() {
            stream.pos += 1;

            if ch == b'\n' {
                end_pos = if last_ch == b'\r' {
                    stream.pos - 2
                } else {
                    stream.pos - 1
                };
            } else if ch == b'-'
                && matches!(iter.peek(), Some(b'-'))
                && stream
                    .data
                    .get(stream.pos + 1..stream.pos + 1 + boundary.len())
                    == Some(boundary)
                && matches!(
                    stream.data.get(stream.pos + 1 + boundary.len()..),
                    Some([b'\n' | b'\r' | b' ' | b'\t', ..])
                        | Some([b'-', b'-', ..])
                        | Some([])
                        | None
                )
            {
                if last_ch != b'\n' {
                    end_pos = stream.pos - 1;
                }
                stream.pos += boundary.len() + 1;
                return (end_pos, true);
            }

            last_ch = ch;
        }

        (stream.pos, false)
    } else {
        stream.pos = stream.data.len();
        (stream.pos, true)
    }
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
pub fn seek_crlf(stream: &MessageStream, mut start_pos: usize) -> usize {
    for ch in &stream.data[start_pos..] {
        match ch {
            b'\r' | b' ' | b'\t' => start_pos += 1,
            b'\n' => {
                start_pos += 1;
                break;
            }
            _ => break,
        }
    }

    start_pos
}
