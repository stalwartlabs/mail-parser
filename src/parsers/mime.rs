use std::borrow::Cow;

use super::message::MessageStream;

pub fn seek_next_part(stream: &mut MessageStream, boundary: &[u8]) -> bool {
    if !boundary.is_empty() {
        let mut pos = stream.pos;

        let mut match_count = 0;

        for ch in stream.data[pos..].iter() {
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
) -> (usize, Option<Cow<'x, [u8]>>) {
    let mut read_pos = start_pos;

    return if !boundary.is_empty() {
        let mut match_count = 0;

        for ch in stream.data[read_pos..].iter() {
            read_pos += 1;

            if ch == &boundary[match_count] {
                match_count += 1;
                if match_count == boundary.len() {
                    if is_boundary_end(stream, read_pos) {
                        let match_pos = read_pos - match_count;

                        return (
                            read_pos - start_pos,
                            if start_pos < match_pos {
                                Cow::from(&stream.data[start_pos..match_pos]).into()
                            } else {
                                None
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

        (0, None)
    } else if start_pos < stream.data.len() {
        (
            stream.data.len() - start_pos,
            Cow::from(&stream.data[start_pos..]).into(),
        )
    } else {
        (0, None)
    };
}

#[inline(always)]
pub fn is_boundary_end(stream: &MessageStream, pos: usize) -> bool {
    matches!(
        stream.data.get(pos..),
        Some([b'\n' | b'\r' | b' ' | b'\t', ..]) | Some([b'-', b'-', ..]) | Some([]) | None
    )
}

pub fn skip_multipart_end(stream: &mut MessageStream) -> bool {
    let pos = stream.pos;

    match stream.data.get(pos..pos + 2) {
        Some(b"--") => {
            if let Some(byte) = stream.data.get(pos + 2) {
                if !(*byte).is_ascii_whitespace() {
                    return false;
                }
            }
            stream.pos = pos + 2;
            true
        }
        _ => false,
    }
}

#[inline(always)]
pub fn skip_crlf(stream: &mut MessageStream) {
    let mut pos = stream.pos;

    for ch in stream.data[pos..].iter() {
        match ch {
            b'\r' | b' ' | b'\t' => pos += 1,
            b'\n' => {
                stream.pos = pos + 1;
                break;
            }
            _ => break,
        }
    }
}
