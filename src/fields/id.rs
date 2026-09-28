/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::FieldCtx;
use crate::store::Value;
use memchr::memchr2_iter;
use std::ops::Range;

#[inline]
pub(crate) fn parse_id(ctx: &mut FieldCtx<'_>, value: Range<usize>) -> Value {
    let bytes = ctx.bytes(value.clone());
    let base = value.start;
    let mark = ctx.text_items_mark();
    let mut open = None;
    for at in memchr2_iter(b'<', b'>', bytes) {
        match (open, bytes.get(at)) {
            (None, Some(b'<')) => open = Some(at + 1),
            (Some(start), Some(b'>')) => {
                if let Some(id) = trim(bytes, start..at, is_id_byte) {
                    let item = ctx.borrow(base + id.start..base + id.end);
                    ctx.push_text_item(item);
                }
                open = None;
            }
            _ => (),
        }
    }
    if ctx.text_items_mark() == mark
        && let Some(bare) = bare_span(bytes)
    {
        let item = ctx.borrow(base + bare.start..base + bare.end);
        ctx.push_text_item(item);
    }
    ctx.text_list(mark)
}

fn is_id_byte(byte: &u8) -> bool {
    !matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'<')
}

fn is_bare_byte(byte: &u8) -> bool {
    !matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'>')
}

fn trim(bytes: &[u8], range: Range<usize>, keep: fn(&u8) -> bool) -> Option<Range<usize>> {
    let span = bytes.get(range.clone())?;
    let first = span.iter().position(keep)?;
    let last = span.iter().rposition(keep)?;
    Some(range.start + first..range.start + last + 1)
}

fn bare_span(bytes: &[u8]) -> Option<Range<usize>> {
    let mut bare: Option<Range<usize>> = None;
    let mut extend = |segment: Range<usize>| {
        if let Some(span) = trim(bytes, segment, is_bare_byte) {
            bare = Some(bare.take().map_or(span.start, |bare| bare.start)..span.end);
        }
    };
    let mut outside = Some(0);
    for at in memchr2_iter(b'<', b'>', bytes) {
        match (outside, bytes.get(at)) {
            (Some(start), Some(b'<')) => {
                extend(start..at);
                outside = None;
            }
            (None, Some(b'>')) => outside = Some(at + 1),
            _ => (),
        }
    }
    if let Some(start) = outside {
        extend(start..bytes.len());
    }
    bare
}

#[cfg(test)]
pub(crate) mod tests {
    use super::parse_id;
    use crate::{
        HeaderForm, HeaderValue,
        fields::{FieldCtx, tests::load_tests},
        store::{MessageData, Value},
    };
    use serde_json::Value as Json;

    pub(crate) type Items = Option<Vec<String>>;
    pub(crate) type Case = (&'static [u8], Option<&'static [&'static str]>);

    const EDGE_CASES: &[Case] = &[
        (b"<a\xff@b>\n", Some(&["a\u{fffd}@b"])),
        (b"x\xff <\n", Some(&["x\u{fffd}"])),
    ];

    pub(crate) fn items(value: HeaderValue<'_>) -> Items {
        match value {
            HeaderValue::Empty => None,
            HeaderValue::TextList(list) => Some(list.iter().map(str::to_string).collect()),
            other => Some(vec![format!("unexpected value shape {other:?}")]),
        }
    }

    pub(crate) fn expected_items(expected: &Json) -> Items {
        expected.as_array().map(|list| {
            list.iter()
                .map(|item| item.as_str().unwrap_or_default().to_string())
                .collect()
        })
    }

    pub(crate) fn case_items(expected: Option<&[&str]>) -> Items {
        expected.map(|list| list.iter().map(|item| item.to_string()).collect())
    }

    fn parse(input: &[u8]) -> Items {
        items(HeaderForm::MessageIds.parse(input).value())
    }

    #[test]
    fn message_id_fixtures() {
        let tests = load_tests("id.json");
        assert_eq!(tests.len(), 34);
        for (header, expected) in tests {
            assert_eq!(
                parse(header.as_bytes()),
                expected_items(&expected),
                "{header:?}"
            );
        }
    }

    #[test]
    fn message_id_edge_cases() {
        for &(input, expected) in EDGE_CASES {
            assert_eq!(
                parse(input),
                case_items(expected),
                "{:?}",
                String::from_utf8_lossy(input)
            );
        }
        let parsed = HeaderForm::MessageIds.parse(b" <single@id>\r\n");
        assert!(matches!(parsed.value(), HeaderValue::TextList(list) if list.len() == 1));
        let parsed = HeaderForm::MessageIds.parse(b" <a@b> <c@d>\r\n");
        assert_eq!(parsed.value().as_text(), Some("c@d"));
    }

    #[test]
    fn message_ids_borrow_unless_lossy() {
        let source = b"Message-ID: <abc@host>\r\nReferences: <x\xff@y> <z@w>\r\n";
        let mut data = MessageData::default();
        assert!(matches!(
            parse_id(&mut FieldCtx::new(source, &mut data), 11..24),
            Value::TextList(_)
        ));
        assert!(data.strings.is_empty());
        assert!(matches!(
            parse_id(&mut FieldCtx::new(source, &mut data), 36..source.len()),
            Value::TextList(_)
        ));
        assert_eq!(data.strings, "x\u{fffd}@y");
    }
}
