/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use memchr::{memchr, memchr2};

use super::decode_entity;

const COMMENT_OPEN: &[u8] = b"--";
const COMMENT_CLOSE: &[u8] = b"--";
const COMMENT_BANG_CLOSE: &[u8] = b"--!";
const NAMED_TAG: usize = 1;

static TEXT_DELIMITERS: [bool; 256] = delimiters(b"<&; \t\r\n");
static TAG_DELIMITERS: [bool; 256] = delimiters(b"<>/ \t\r\n");

const fn delimiters(bytes: &[u8]) -> [bool; 256] {
    let mut table = [false; 256];
    let mut index = 0;
    while index < bytes.len() {
        table[bytes[index] as usize] = true;
        index += 1;
    }
    table
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tag {
    Break,
    Paragraph,
    Head,
    Style,
    Script,
    Template,
}

impl Tag {
    fn parse(name: &[u8]) -> Option<Tag> {
        hashify::map_ignore_case!(name, Tag,
            "br" => Tag::Break,
            "p" => Tag::Paragraph,
            "head" => Tag::Head,
            "style" => Tag::Style,
            "script" => Tag::Script,
            "template" => Tag::Template,
        )
        .copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Complete,
    Preview { limit: usize },
}

pub(super) fn html_to_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    convert(input, &mut out, Mode::Complete);
    out
}

pub(crate) fn html_to_text_prefix(input: &str, limit: usize, out: &mut String) -> bool {
    convert(input, out, Mode::Preview { limit })
}

fn convert(input: &str, out: &mut String, mode: Mode) -> bool {
    let bytes = input.as_bytes();
    let mut sink = Sink {
        input,
        bytes,
        out,
        mode,
        pending_start: 0,
        pending_end: 0,
    };
    let mut in_tag = false;
    let mut in_head = false;
    let mut in_style = false;
    let mut in_script = false;
    let mut in_template = false;
    let mut is_token_start = true;
    let mut is_after_space = false;
    let mut is_tag_close = false;
    let mut is_new_line = true;
    let mut token_start = 0;
    let mut token_end = 0;
    let mut tag_token_pos = 0;
    let mut pos = 0;

    loop {
        let hidden = in_head || in_style || in_script || in_template;
        if in_tag {
            if tag_token_pos > NAMED_TAG {
                pos = find(bytes, pos, |rest| memchr2(b'<', b'>', rest));
            } else if !is_token_start {
                let run = run_until(bytes, pos, &TAG_DELIMITERS);
                if run > 0 {
                    pos += run;
                    token_end = pos - 1;
                }
            }
        } else if hidden {
            pos = find(bytes, pos, |rest| memchr(b'<', rest));
        } else {
            let run = run_until(bytes, pos, &TEXT_DELIMITERS);
            if run > 0 {
                if is_token_start {
                    token_start = pos;
                    is_token_start = false;
                }
                pos += run;
                token_end = pos - 1;
            }
        }
        let Some(&ch) = bytes.get(pos) else {
            break;
        };
        match ch {
            b'<' => {
                if !in_tag && !hidden && !is_token_start {
                    if sink.emit(token_start, token_end + 1, is_after_space) {
                        return true;
                    }
                    is_after_space = false;
                }
                tag_token_pos = 0;
                in_tag = true;
                is_token_start = true;
                is_tag_close = false;
                pos += 1;
                continue;
            }
            b'>' if in_tag => {
                if tag_token_pos >= NAMED_TAG
                    && let Some(tag) = bytes.get(token_start..token_end + 1)
                {
                    match Tag::parse(tag) {
                        Some(found @ (Tag::Break | Tag::Paragraph))
                            if found == Tag::Break || is_tag_close =>
                        {
                            if sink.newline() {
                                return true;
                            }
                            is_after_space = false;
                            is_new_line = true;
                        }
                        Some(Tag::Head) => in_head = !is_tag_close,
                        Some(Tag::Style) => in_style = !is_tag_close,
                        Some(Tag::Script) => in_script = !is_tag_close,
                        Some(Tag::Template) => in_template = !is_tag_close,
                        Some(Tag::Break | Tag::Paragraph) | None => {}
                    }
                }
                in_tag = false;
                is_token_start = true;
                pos += 1;
                continue;
            }
            b'/' if in_tag => {
                if tag_token_pos == 0 {
                    is_tag_close = true;
                }
                pos += 1;
                continue;
            }
            b'!' if in_tag
                && tag_token_pos == 0
                && bytes.get(pos + 1..pos + 1 + COMMENT_OPEN.len()) == Some(COMMENT_OPEN) =>
            {
                let Some(close) = comment_close(bytes, pos + 1) else {
                    break;
                };
                in_tag = false;
                is_token_start = true;
                pos = close + 1;
                continue;
            }
            b' ' | b'\t' | b'\r' | b'\n' => {
                if !in_tag && !hidden {
                    if !is_token_start {
                        if sink.emit(token_start, token_end + 1, is_after_space && !is_new_line) {
                            return true;
                        }
                        is_new_line = false;
                    }
                    is_after_space = true;
                }
                is_token_start = true;
                pos += 1;
                continue;
            }
            b'&' if !in_tag && !is_token_start && !in_head => {
                if in_style || in_script || in_template {
                    pos += 1;
                    continue;
                }
                if sink.emit(token_start, token_end + 1, is_after_space && !is_new_line) {
                    return true;
                }
                is_new_line = false;
                is_token_start = true;
                is_after_space = false;
            }
            b';' if !in_tag && !is_token_start && !in_head => {
                if in_style || in_script || in_template {
                    pos += 1;
                    continue;
                }
                if sink.emit(token_start, pos + 1, is_after_space && !is_new_line) {
                    return true;
                }
                is_token_start = true;
                is_after_space = false;
                is_new_line = false;
                pos += 1;
                continue;
            }
            _ => (),
        }
        if is_token_start {
            is_token_start = false;
            if in_tag {
                tag_token_pos += 1;
                if tag_token_pos > NAMED_TAG {
                    pos += 1;
                    continue;
                }
            }
            token_start = pos;
        }
        token_end = pos;
        pos += 1;
    }

    if !in_tag && !is_token_start && !(in_head || in_style || in_script || in_template) {
        sink.emit(token_start, token_end + 1, is_after_space && !is_new_line);
    }
    sink.flush();
    false
}

fn find(bytes: &[u8], pos: usize, search: impl Fn(&[u8]) -> Option<usize>) -> usize {
    bytes
        .get(pos..)
        .and_then(search)
        .map_or(bytes.len(), |offset| pos + offset)
}

fn run_until(bytes: &[u8], pos: usize, delimiters: &[bool; 256]) -> usize {
    let rest = bytes.get(pos..).unwrap_or_default();
    rest.iter()
        .position(|&byte| delimiters[usize::from(byte)])
        .unwrap_or(rest.len())
}

fn comment_close(bytes: &[u8], comment_start: usize) -> Option<usize> {
    let mut pos = comment_start;
    loop {
        let close = pos + memchr(b'>', bytes.get(pos..)?)?;
        let comment = bytes.get(comment_start..close)?;
        if comment.ends_with(COMMENT_CLOSE)
            || comment
                .strip_prefix(COMMENT_OPEN)
                .is_some_and(|body| body.ends_with(COMMENT_BANG_CLOSE))
        {
            return Some(close);
        }
        pos = close + 1;
    }
}

struct Sink<'x, 'o> {
    input: &'x str,
    bytes: &'x [u8],
    out: &'o mut String,
    mode: Mode,
    pending_start: usize,
    pending_end: usize,
}

impl Sink<'_, '_> {
    fn emit(&mut self, start: usize, end: usize, add_space: bool) -> bool {
        let Some(token) = self.input.get(start..end) else {
            return false;
        };
        if let Some(ch) = decode_entity(token) {
            self.flush();
            if add_space {
                self.out.push(' ');
            }
            if ch != '\r' || self.mode == Mode::Complete {
                self.out.push(ch);
            }
            return self.limit_reached();
        }
        let span_start = match start.checked_sub(1) {
            Some(space) if add_space && self.bytes.get(space) == Some(&b' ') => space,
            _ if add_space => {
                self.flush();
                self.out.push(' ');
                start
            }
            _ => start,
        };
        if self.pending_start < self.pending_end && self.pending_end == span_start {
            self.pending_end = end;
        } else {
            self.flush();
            self.pending_start = span_start;
            self.pending_end = end;
        }
        self.limit_reached()
    }

    fn newline(&mut self) -> bool {
        self.flush();
        self.out.push('\n');
        self.limit_reached()
    }

    fn limit_reached(&mut self) -> bool {
        let Mode::Preview { limit } = self.mode else {
            return false;
        };
        let reached = self.out.len() + (self.pending_end - self.pending_start) > limit;
        if reached {
            self.flush();
        }
        reached
    }

    fn flush(&mut self) {
        if let Some(pending) = self.input.get(self.pending_start..self.pending_end) {
            self.out.push_str(pending);
        }
        self.pending_start = self.pending_end;
    }
}
