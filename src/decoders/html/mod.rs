/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! HTML to text, text to HTML, and HTML charset declarations.

mod entities;
mod meta;
mod text;

use std::char::REPLACEMENT_CHARACTER;

use memchr::{memchr_iter, memchr2, memchr3};

pub use meta::strip_charset_meta;
pub(crate) use text::html_to_text_prefix;

const HTML_PREFIX: &str = "<html><body>";
const HTML_SUFFIX: &str = "</body></html>";
const LINE_BREAK: &str = "<br/>";

/// Converts HTML to plain text: tags are removed, entities decoded and
/// white space collapsed; `<br>` and `</p>` become line breaks; the content
/// of `<head>`, `<style>`, `<script>` and `<template>` is skipped.
pub fn html_to_text(input: &str) -> String {
    text::html_to_text(input)
}

/// Appends one text token of an HTML document to `result`, decoded when it
/// is an entity (`&amp;`, `&#233;`), after a space when `add_space` is set.
/// Invalid UTF-8 becomes U+FFFD. For callers with their own HTML tokenizer.
pub fn add_html_token(result: &mut String, token: &[u8], add_space: bool) {
    let token = String::from_utf8_lossy(token);
    if add_space {
        result.push(' ');
    }
    match decode_entity(&token) {
        Some(ch) => result.push(ch),
        None => result.push_str(&token),
    }
}

/// Converts plain text to HTML: `<`, `>` and `&` are escaped, line breaks
/// become `<br/>`, carriage returns are dropped, and the result is wrapped
/// in `<html><body>`.
pub fn text_to_html(input: &str) -> String {
    let bytes = input.as_bytes();
    let line_breaks = memchr_iter(b'\n', bytes).count();
    let mut result = String::with_capacity(
        bytes.len() + line_breaks * (LINE_BREAK.len() - 1) + HTML_PREFIX.len() + HTML_SUFFIX.len(),
    );
    result.push_str(HTML_PREFIX);
    let find_common = |from: usize| find(bytes, from, |rest| memchr3(b'\n', b'\r', b'<', rest));
    let find_rare = |from: usize| find(bytes, from, |rest| memchr2(b'>', b'&', rest));
    let mut next_common = find_common(0);
    let mut next_rare = find_rare(0);
    let mut literal_start = 0;
    while let Some(special) = earliest(next_common, next_rare) {
        result.push_str(input.get(literal_start..special).unwrap_or_default());
        result.push_str(match bytes.get(special) {
            Some(b'\n') => LINE_BREAK,
            Some(b'<') => "&lt;",
            Some(b'>') => "&gt;",
            Some(b'&') => "&amp;",
            _ => "",
        });
        literal_start = special + 1;
        if next_common == Some(special) {
            next_common = find_common(literal_start);
        }
        if next_rare == Some(special) {
            next_rare = find_rare(literal_start);
        }
    }
    result.push_str(input.get(literal_start..).unwrap_or_default());
    result.push_str(HTML_SUFFIX);
    result
}

fn find(bytes: &[u8], from: usize, search: impl Fn(&[u8]) -> Option<usize>) -> Option<usize> {
    bytes
        .get(from..)
        .and_then(search)
        .map(|offset| from + offset)
}

fn earliest(first: Option<usize>, second: Option<usize>) -> Option<usize> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.min(second)),
        (first, second) => first.or(second),
    }
}

fn decode_entity(token: &str) -> Option<char> {
    let entity = token.strip_prefix('&')?.strip_suffix(';')?;
    let code = match entity.strip_prefix('#') {
        Some(number) => match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse::<u32>().ok()?,
        },
        None => entities::lookup(entity.as_bytes()).copied()?,
    };
    Some(char::from_u32(code).unwrap_or(REPLACEMENT_CHARACTER))
}

#[cfg(test)]
mod tests {
    use super::{add_html_token, html_to_text, strip_charset_meta, text_to_html};
    use crate::fields::tests::load_text_pairs;
    use std::borrow::Cow;

    #[test]
    fn text_to_html_fixtures() {
        let tests = load_text_pairs("decoders/text_to_html.json");
        assert_eq!(tests.len(), 3);
        for (input, expected) in tests {
            assert_eq!(text_to_html(&input), expected, "{input:?}");
        }
    }

    #[test]
    fn html_to_text_fixtures() {
        let tests = load_text_pairs("decoders/html_to_text.json");
        assert_eq!(tests.len(), 24);
        for (input, expected) in tests {
            assert_eq!(html_to_text(&input), expected, "{input:?}");
        }
    }

    #[test]
    fn html_entity_fixtures() {
        let tests = load_text_pairs("decoders/html_entities.json");
        assert_eq!(tests.len(), 9);
        for (input, expected) in tests {
            let mut result = String::with_capacity(input.len());
            add_html_token(&mut result, input.as_bytes(), false);
            assert_eq!(result, expected, "{input:?}");
        }
    }

    #[test]
    fn charset_meta_fixtures() {
        let tests = load_text_pairs("decoders/charset_meta.json");
        assert_eq!(tests.len(), 7);
        for (input, expected) in tests {
            let stripped = strip_charset_meta(&input);
            assert_eq!(
                matches!(stripped, Cow::Borrowed(_)),
                input == expected,
                "{input:?}"
            );
            assert_eq!(stripped, expected, "{input:?}");
        }
    }
}
