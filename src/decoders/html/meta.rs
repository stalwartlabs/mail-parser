/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{borrow::Cow, ops::Range};

use memchr::{memchr, memchr_iter, memchr2_iter};

const UTF_8: &str = "utf-8";
const BODY: &[u8] = b"body";
const META: &[u8] = b"meta";
const CHARSET: &[u8] = b"charset";
const QUOTE_ENTITY: &[u8] = b"&quot;";

/// Rewrites the charset of the `<meta charset>` and `<meta
/// http-equiv="Content-Type">` declarations before `<body` to `utf-8` (issue
/// #109), so that a browser does not decode HTML that is already UTF-8 a
/// second time. Borrowed when there is nothing to rewrite.
///
/// ```
/// use mail_parser::strip_charset_meta;
///
/// let html = r#"<html><head><meta charset="iso-8859-1"></head><body>caf\u{e9}</body></html>"#;
/// assert_eq!(
///     strip_charset_meta(html),
///     r#"<html><head><meta charset="utf-8"></head><body>caf\u{e9}</body></html>"#
/// );
/// ```
pub fn strip_charset_meta(html: &str) -> Cow<'_, str> {
    let bytes = html.as_bytes();
    let mut result: Option<String> = None;
    let mut copied = 0;
    for tag_start in memchr_iter(b'<', bytes) {
        let name = bytes.get(tag_start + 1..).unwrap_or_default();
        if starts_with_ignore_case(name, BODY) {
            break;
        }
        if !is_meta_tag(name) {
            continue;
        }
        let tag_end = memchr(b'>', name).map_or(bytes.len(), |end| tag_start + 1 + end);
        for value in charset_values(bytes, tag_start..tag_end) {
            if value.start < copied {
                continue;
            }
            let out = result.get_or_insert_with(|| String::with_capacity(html.len()));
            out.push_str(html.get(copied..value.start).unwrap_or_default());
            out.push_str(UTF_8);
            copied = value.end;
        }
    }
    match result {
        Some(mut out) => {
            out.push_str(html.get(copied..).unwrap_or_default());
            Cow::Owned(out)
        }
        None => Cow::Borrowed(html),
    }
}

fn is_meta_tag(name: &[u8]) -> bool {
    starts_with_ignore_case(name, META)
        && name
            .get(META.len())
            .is_none_or(|&byte| byte.is_ascii_whitespace() || byte == b'/' || byte == b'>')
}

fn charset_values(bytes: &[u8], tag: Range<usize>) -> impl Iterator<Item = Range<usize>> {
    let tag_bytes = bytes.get(tag.clone()).unwrap_or_default();
    memchr2_iter(b'c', b'C', tag_bytes).filter_map(move |offset| {
        let name_end = offset + CHARSET.len();
        if !tag_bytes
            .get(offset..name_end)
            .is_some_and(|name| name.eq_ignore_ascii_case(CHARSET))
        {
            return None;
        }
        let value = value_range(tag_bytes, name_end)?;
        let text = tag_bytes.get(value.clone())?;
        (!text.is_empty()
            && !text.eq_ignore_ascii_case(b"utf-8")
            && !text.eq_ignore_ascii_case(b"utf8"))
        .then_some(tag.start + value.start..tag.start + value.end)
    })
}

fn value_range(tag: &[u8], name_end: usize) -> Option<Range<usize>> {
    let after_name = skip_whitespace(tag, name_end);
    let rest = tag.get(after_name..)?.strip_prefix(b"=")?;
    let start = skip_whitespace(tag, tag.len() - rest.len());
    let rest = tag.get(start..)?;
    Some(match rest {
        [quote @ (b'"' | b'\''), value @ ..] => {
            start + 1..start + 1 + memchr(*quote, value).unwrap_or(value.len())
        }
        _ if starts_with_ignore_case(rest, QUOTE_ENTITY) => {
            let value = rest.get(QUOTE_ENTITY.len()..).unwrap_or_default();
            let len = value
                .iter()
                .position(|&byte| byte == b'&')
                .filter(|&end| {
                    starts_with_ignore_case(value.get(end..).unwrap_or_default(), QUOTE_ENTITY)
                })
                .unwrap_or_else(|| unquoted_len(value));
            start + QUOTE_ENTITY.len()..start + QUOTE_ENTITY.len() + len
        }
        _ => start..start + unquoted_len(rest),
    })
}

fn unquoted_len(value: &[u8]) -> usize {
    value
        .iter()
        .position(|&byte| {
            byte.is_ascii_whitespace() || matches!(byte, b';' | b'"' | b'\'' | b'>' | b'/' | b'&')
        })
        .unwrap_or(value.len())
}

fn skip_whitespace(bytes: &[u8], from: usize) -> usize {
    from + bytes
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|byte| byte.is_ascii_whitespace())
        .count()
}

fn starts_with_ignore_case(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes
        .get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}
