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

    use std::borrow::Cow;

    use super::{add_html_token, html_to_text, strip_charset_meta, text_to_html};

    #[test]
    fn convert_text_to_html() {
        let inputs = [
            (
                "hello\nworld\n",
                "<html><body>hello<br/>world<br/></body></html>",
            ),
            (
                "using <>\n",
                "<html><body>using &lt;&gt;<br/></body></html>",
            ),
            ("a & b\r\n", "<html><body>a &amp; b<br/></body></html>"),
        ];

        for input in inputs {
            assert_eq!(text_to_html(input.0), input.1);
        }
    }

    #[test]
    fn convert_html_to_text() {
        let inputs = [
            ("<html>hello<br/>world<br/></html>", "hello\nworld\n"),
            ("<html>using &lt;><br/></html>", "using <>\n"),
            ("test <not br/>tag<br />", "test tag\n"),
            ("<>< ><tag\n/>>hello    world< br \n />", ">hello world\n"),
            (
                concat!(
                    "<head><title>ignore head</title><not head>xyz</not head></head>",
                    "<h1>&lt;body&gt;</h1>"
                ),
                "<body>",
            ),
            (
                concat!(
                    "<p>what is &heartsuit;?</p><p>&#x000DF;&Abreve;&#914;&gamma; ",
                    "don&apos;t hurt me.</p>"
                ),
                "what is ♥?\nßĂΒγ don't hurt me.\n",
            ),
            (
                concat!(
                    "<!--[if mso]><style type=\"text/css\">body, table, td, a, p, ",
                    "span, ul, li {font-family: Arial, sans-serif!important;}</style><![endif]-->",
                    "this is <!-- <> < < < < ignore  > -> here -->the actual<!--> text"
                ),
                "this is the actual text",
            ),
            (
                "   < p >  hello < / p > < p > world < / p >   !!! < br > ",
                "hello\nworld\n!!!\n",
            ),
            (
                " <p>please unsubscribe <a href=#>here</a>.</p> ",
                "please unsubscribe here.\n",
            ),
        ];

        for input in inputs {
            assert_eq!(html_to_text(input.0), input.1, "Failed for '{:?}'", input.0);
        }
    }

    #[test]
    fn convert_html_entity() {
        let inputs = [
            ("&lt;", "<"),
            ("&#32;", " "),
            ("&#x20;", " "),
            ("&nbsp;", "\u{a0}"),
            ("&CounterClockwiseContourIntegral;", "∳"),
            ("&curvearrowright;", "↷"),
            ("&rarr;", "→"),
            ("&#xFFFFFFF;", "�"),
            ("&hmmm", "&hmmm"),
        ];

        for input in inputs {
            let mut result = String::with_capacity(input.0.len());
            add_html_token(&mut result, input.0.as_bytes(), false);
            assert_eq!(result, input.1, "Failed for '{:?}", input.0);
        }
    }

    #[test]
    fn html_to_text_removes_style_content() {
        let input = "<style>body{color:red}</style><div>Hello</div>";
        let output = html_to_text(input);
        assert!(!output.contains("body{color:red}"));
        assert!(output.contains("Hello"));
    }

    #[test]
    fn html_to_text_removes_script_content() {
        let input = concat!(
            "<style>body{color:red}</style>",
            "<div>Hello</div>",
            "<script>alert('x')</script>",
            "<div>World</div>"
        );
        let output = html_to_text(input);
        assert!(!output.contains("body{color:red}"));
        assert!(!output.contains("alert('x')"));
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn html_to_text_removes_template_content() {
        let input = concat!(
            "<div>Hello</div>",
            "<template><div>Hidden</div><style>.x{}</style></template>",
            "<div>World</div>"
        );
        let output = html_to_text(input);
        assert!(!output.contains("Hidden"));
        assert!(!output.contains(".x{}"));
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn charset_meta_is_rewritten() {
        for (html, expected) in [
            (
                "<head><meta charset=Windows-1252></head><body>\u{2014}</body>",
                "<head><meta charset=utf-8></head><body>\u{2014}</body>",
            ),
            (
                "<head><meta charset=\"Windows-1252\"></head><body>\u{2014}</body>",
                "<head><meta charset=\"utf-8\"></head><body>\u{2014}</body>",
            ),
            (
                "<head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=Windows-1252\"></head><body>\u{2014}</body>",
                "<head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\"></head><body>\u{2014}</body>",
            ),
            (
                "<head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=&quot;Windows-1252&quot;\"></head><body>\u{2014}</body>",
                "<head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=&quot;utf-8&quot;\"></head><body>\u{2014}</body>",
            ),
        ] {
            assert_eq!(strip_charset_meta(html), expected);
        }
        for html in [
            "<head><meta charset=\"utf-8\"></head><body>x</body>",
            "<body><meta charset=Windows-1252></body>",
            "plain text",
        ] {
            assert!(matches!(strip_charset_meta(html), Cow::Borrowed(_)));
        }
    }
}
