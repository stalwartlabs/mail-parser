/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Previews and truncation of text and HTML bodies.

use std::borrow::Cow;

use memchr::memchr;

use super::html::html_to_text_prefix;

const ELLIPSIS: &str = "...";
const MIN_LEN_WITH_ELLIPSIS: usize = 7;
const COMMENT_OPEN: &[u8] = b"!--";
const COMMENT_CLOSE: &[u8] = b"--";
const LOOKAHEAD: usize = 4;
const PREVIEW_SLACK: usize = 64;

/// At most `max_len` bytes of `text`, carriage returns removed. Longer
/// text is cut at a character boundary and, when `max_len` is 7 or more,
/// ends with `...` (counted in `max_len`). Borrowed when the text fits and
/// has no carriage return.
///
/// ```
/// use mail_parser::preview_text;
///
/// assert_eq!(preview_text("Hello,\r\nworld", 20), "Hello,\nworld");
/// assert_eq!(preview_text("Hello, world", 8), "Hello...");
/// ```
pub fn preview_text(text: &str, max_len: usize) -> Cow<'_, str> {
    match strip_cr_prefix(text, max_len.saturating_add(1)) {
        Stripped::Complete(text) if text.len() <= max_len => text,
        Stripped::Complete(text) | Stripped::Prefix(text) => Cow::Owned(match text {
            Cow::Borrowed(text) => truncate_chars(text, max_len),
            Cow::Owned(text) => truncate_owned(text, max_len),
        }),
    }
}

/// A text preview of an HTML document: [`crate::html_to_text`] of the
/// document cut like [`preview_text`]. Only the part of the document that
/// the preview needs is converted.
pub fn preview_html(html: &str, max_len: usize) -> String {
    preview_html_prefix(html, max_len, true).unwrap_or_default()
}

pub(crate) fn preview_html_prefix(html: &str, max_len: usize, complete: bool) -> Option<String> {
    let mut text = String::with_capacity(max_len.saturating_add(PREVIEW_SLACK).min(html.len()));
    if !html_to_text_prefix(html, max_len, &mut text) && !complete {
        return None;
    }
    if text.len() > max_len {
        let (keep, ellipsis) = cut(max_len);
        text.truncate(text.floor_char_boundary(keep));
        text.extend(ellipsis);
    }
    Some(text)
}

/// Truncates text for display; the same as [`preview_text`].
pub fn truncate_text(text: &str, max_len: usize) -> Cow<'_, str> {
    preview_text(text, max_len)
}

/// At most `max_len` bytes of an HTML document, carriage returns removed.
/// A longer document is cut before a tag rather than inside one and, when
/// `max_len` is 7 or more, ends with `...`. Borrowed when the document fits
/// and has no carriage return.
pub fn truncate_html(html: &str, max_len: usize) -> Cow<'_, str> {
    match strip_cr_prefix(html, max_len.saturating_add(LOOKAHEAD)) {
        Stripped::Complete(html) if html.len() <= max_len => html,
        Stripped::Complete(html) | Stripped::Prefix(html) => {
            Cow::Owned(truncate_markup(&html, max_len))
        }
    }
}

enum Stripped<'x> {
    Complete(Cow<'x, str>),
    Prefix(Cow<'x, str>),
}

fn strip_cr_prefix(text: &str, needed: usize) -> Stripped<'_> {
    let window = text.as_bytes().get(..needed).unwrap_or(text.as_bytes());
    if memchr(b'\r', window).is_none() {
        return if text.len() < needed {
            Stripped::Complete(Cow::Borrowed(text))
        } else {
            Stripped::Prefix(Cow::Borrowed(text))
        };
    }
    let mut stripped = String::with_capacity(needed.min(text.len()));
    let mut rest = text;
    while stripped.len() < needed {
        let missing = needed - stripped.len();
        let window = rest.as_bytes().get(..missing).unwrap_or(rest.as_bytes());
        match memchr(b'\r', window) {
            Some(cr) => {
                let (segment, tail) = rest.split_at(cr);
                stripped.push_str(segment);
                rest = tail.get(1..).unwrap_or_default();
            }
            None if rest.len() <= missing => {
                stripped.push_str(rest);
                return Stripped::Complete(Cow::Owned(stripped));
            }
            None => {
                stripped.push_str(
                    rest.get(..rest.ceil_char_boundary(missing))
                        .unwrap_or_default(),
                );
                return Stripped::Prefix(Cow::Owned(stripped));
            }
        }
    }
    Stripped::Prefix(Cow::Owned(stripped))
}

fn cut(max_len: usize) -> (usize, Option<&'static str>) {
    if max_len >= MIN_LEN_WITH_ELLIPSIS {
        (max_len - ELLIPSIS.len(), Some(ELLIPSIS))
    } else {
        (max_len, None)
    }
}

fn truncate_chars(text: &str, max_len: usize) -> String {
    let (keep, ellipsis) = cut(max_len);
    let kept = text
        .get(..text.floor_char_boundary(keep))
        .unwrap_or_default();
    let mut result = String::with_capacity(kept.len() + ELLIPSIS.len());
    result.push_str(kept);
    result.extend(ellipsis);
    result
}

fn truncate_owned(mut text: String, max_len: usize) -> String {
    let (keep, ellipsis) = cut(max_len);
    text.truncate(text.floor_char_boundary(keep));
    text.extend(ellipsis);
    text
}

fn truncate_markup(html: &str, max_len: usize) -> String {
    let (keep, ellipsis) = cut(max_len);
    let bytes = html.as_bytes();
    let mut in_tag = false;
    let mut in_comment = false;
    let mut last_tag_end = 0;
    let mut end = html.len();
    for (pos, ch) in html.char_indices() {
        let mut tag_boundary = 0;
        match ch {
            '<' if !in_tag => {
                in_tag = true;
                if bytes.get(pos + 1..pos + 1 + COMMENT_OPEN.len()) == Some(COMMENT_OPEN) {
                    in_comment = true;
                }
                tag_boundary = pos;
            }
            '>' if in_tag => {
                if !in_comment {
                    in_tag = false;
                    tag_boundary = pos + 1;
                } else if pos
                    .checked_sub(COMMENT_CLOSE.len())
                    .and_then(|start| bytes.get(start..pos))
                    == Some(COMMENT_CLOSE)
                {
                    in_comment = false;
                    in_tag = false;
                    tag_boundary = pos + 1;
                }
            }
            _ => (),
        }
        if ch.len_utf8() + pos > keep {
            end = if (in_tag || tag_boundary > 0) && last_tag_end > 0 {
                last_tag_end
            } else {
                pos
            };
            break;
        } else if tag_boundary > 0 {
            last_tag_end = tag_boundary;
        }
    }
    let kept = html.get(..end).unwrap_or_default();
    let mut result = String::with_capacity(kept.len() + ELLIPSIS.len());
    result.push_str(kept);
    result.extend(ellipsis);
    result
}

#[cfg(test)]
mod tests {
    use crate::fields::tests::{case_text, load_cases};
    use serde_json::Value as Json;
    use std::borrow::Cow;

    fn truncations(name: &str) -> Vec<(String, usize, String)> {
        load_cases(name)
            .iter()
            .map(|case| {
                let max_len = case
                    .get("max_len")
                    .and_then(Json::as_u64)
                    .and_then(|len| usize::try_from(len).ok())
                    .unwrap_or_else(|| panic!("\"max_len\" is a length in {case}"));
                (
                    case_text(case, "input").to_string(),
                    max_len,
                    case_text(case, "expected").to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn truncate_text_fixtures() {
        let tests = truncations("decoders/truncate_text.json");
        assert_eq!(tests.len(), 2);
        for (input, max_len, expected) in tests {
            assert_eq!(super::truncate_text(&input, max_len), expected, "{input:?}");
        }
    }

    #[test]
    fn truncate_html_fixtures() {
        let tests = truncations("decoders/truncate_html.json");
        assert_eq!(tests.len(), 9);
        for (input, max_len, expected) in tests {
            assert_eq!(super::truncate_html(&input, max_len), expected, "{input:?}");
        }
    }

    #[test]
    fn carriage_returns_are_stripped() {
        assert_eq!(
            super::preview_text("hello\r\nworld\r\n", 100),
            "hello\nworld\n"
        );
        assert_eq!(super::preview_text("hello\r\nworld", 8), "hello...");
        assert_eq!(super::preview_text("ab\r\rcd", 4), "abcd");
        assert_eq!(super::preview_text("ab\r\rcde", 4), "abcd");
        assert!(matches!(
            super::preview_text("short", 10),
            Cow::Borrowed("short")
        ));
        assert_eq!(
            super::preview_html("<p>a&#13;b</p>\r\n<p>c</p>", 100),
            "ab\n c\n"
        );
        assert_eq!(super::preview_html("<p>hello world</p>", 8), "hello...");
        assert_eq!(
            super::truncate_html("<p>\r\nhello</p>", 100),
            "<p>\nhello</p>"
        );
    }
}
