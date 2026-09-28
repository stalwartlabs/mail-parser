/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{
    Address, DecodeProblems, HeaderForm, HeaderName, HeaderValue, Message, MessageBuffers,
    MessageParser, PartFlags, PartKind, Source,
    scan::{
        Kernel,
        tests::{fixture_files, with_crlf, with_lf},
    },
};
use encodify::base64;
use std::fmt::Write;

macro_rules! regression {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/eml/regressions/",
            $name,
            ".eml"
        ))
        .as_slice()
    };
}

const MULTIPART: &[u8] = regression!("multipart-basic");
const NESTED_INLINE: &[u8] = regression!("nested-inline-message");
const NESTED_ENCODED: &[u8] = regression!("nested-encoded-message");

fn parse(raw: &[u8]) -> Message<'_> {
    MessageParser::new().parse(raw).expect("message parses")
}

fn value_text(value: HeaderValue<'_>) -> String {
    match value {
        HeaderValue::Empty => "empty".to_string(),
        HeaderValue::Text(text) => format!("text {text:?}"),
        HeaderValue::TextList(list) => format!("list {:?}", list.iter().collect::<Vec<_>>()),
        HeaderValue::Address(list) => list
            .iter()
            .map(|address| match address {
                Address::Mailbox(mailbox) => {
                    format!("<{:?} {:?}>", mailbox.name(), mailbox.address())
                }
                Address::Group(group) => format!("group {:?} {}", group.name(), group.len()),
            })
            .collect(),
        HeaderValue::DateTime(date) => date.to_rfc3339(),
        HeaderValue::ContentType(content_type) => format!(
            "ct {} {:?} {:?}",
            content_type.ctype(),
            content_type.subtype(),
            content_type.attributes().collect::<Vec<_>>()
        ),
        HeaderValue::Received(received) => format!("received {:?}", received.from()),
    }
}

pub(crate) fn dump(message: &Message<'_>) -> String {
    let mut out = String::new();
    for nested in message.messages() {
        let ids = |parts: &mut dyn Iterator<Item = crate::MessagePart<'_>>| {
            parts.map(|part| part.id()).collect::<Vec<_>>()
        };
        let _ = writeln!(
            out,
            "message {} root {} container {:?} source {:?} text {:?} html {:?} attachments {:?}",
            nested.id(),
            nested.root_part().id(),
            nested.container().map(|part| part.id()),
            nested.source(),
            ids(&mut nested.text_body()),
            ids(&mut nested.html_body()),
            ids(&mut nested.attachments()),
        );
    }
    for part in message.parts() {
        let _ = writeln!(
            out,
            "part {} message {} parent {:?} kind {:?} children {:?} encoding {:?} flags {:?} roles {} {} {} {} offsets {} {} {}",
            part.id(),
            part.message().id(),
            part.parent().map(|parent| parent.id()),
            std::mem::discriminant(&part.kind()),
            part.children().map(|child| child.id()).collect::<Vec<_>>(),
            part.encoding(),
            part.flags().names().collect::<Vec<_>>(),
            part.in_text_body(),
            part.in_html_body(),
            part.is_attachment(),
            part.is_inline(),
            part.offset_header(),
            part.offset_body(),
            part.offset_end(),
        );
        for header in part.headers().iter() {
            let _ = writeln!(
                out,
                "  {:?} {:?} {} {} {} {}",
                header.name().as_str(),
                header.raw_name(),
                value_text(header.value()),
                header.offset_field(),
                header.offset_start(),
                header.offset_end()
            );
        }
        let _ = writeln!(out, "  body {:?}", part.decoded());
        if let Some(text) = part.text() {
            let _ = writeln!(out, "  text {text:?}");
        }
    }
    out
}

fn text_of(message: &Message<'_>, part: u32) -> String {
    message
        .part(part)
        .and_then(|part| part.text())
        .map(|text| text.into_owned())
        .unwrap_or_default()
}

#[test]
fn single_part_offsets() {
    let raw = regression!("single-part-offsets");
    let message = parse(raw);
    assert_eq!(message.parts().len(), 1);
    let root = message.root_part();
    assert_eq!(
        (root.offset_header(), root.offset_body(), root.offset_end()),
        (0, 23, raw.len() as u32)
    );
    let headers: Vec<_> = root
        .headers()
        .iter()
        .map(|h| (h.name(), h.offset_field(), h.offset_start(), h.offset_end()))
        .collect();
    assert_eq!(
        headers,
        [
            (HeaderName::From, 0, 5, 10),
            (HeaderName::Subject, 10, 18, 22)
        ]
    );
    assert_eq!(root.raw_body(), b"body\n");
    assert_eq!(message.subject(), Some("Hi"));
    assert!(root.flags().is_empty());
    assert_eq!(message.text_body().map(|p| p.id()).collect::<Vec<_>>(), [0]);
    assert_eq!(message.html_body().map(|p| p.id()).collect::<Vec<_>>(), [0]);
    assert_eq!(message.attachments().len(), 0);
    assert_eq!(message.body_text(0).as_deref(), Some("body\n"));
    assert_eq!(message.root().source(), Source::Raw);
}

#[test]
fn header_block_edge_cases() {
    let message = parse(regression!("header-block-edge-cases"));
    let names: Vec<_> = message
        .headers()
        .iter()
        .map(|h| (h.name().as_str().to_string(), h.raw_name().to_string()))
        .collect();
    assert_eq!(
        names,
        [
            ("Sub ject".to_string(), "Sub ject".to_string()),
            (":Weird".to_string(), ":Weird".to_string()),
            ("baz".to_string(), "baz".to_string()),
            ("X-Custom".to_string(), "X-Custom".to_string()),
        ]
    );
    assert_eq!(
        message.headers().get("x-custom").map(|h| h.raw_value()),
        Some(&b" y\n"[..])
    );
    assert_eq!(
        message.headers().get(":weird").map(|h| h.offset_field()),
        Some(12)
    );
    assert_eq!(message.root_part().raw_body(), b"body");

    let message = parse(b"  \r\n body");
    assert!(message.headers().is_empty());
    assert_eq!(message.root_part().raw_body(), b" body");

    let message = parse(regression!("header-only-no-line-break"));
    assert_eq!(message.subject(), Some("last"));
    assert!(
        message
            .root_part()
            .flags()
            .contains(PartFlags::NO_BLANK_LINE)
    );

    let message = parse(regression!("blank-continuation-in-header-block"));
    assert_eq!(message.headers().len(), 2);
    assert_eq!(message.root_part().raw_body(), b"body");

    let message = parse(b"\n");
    assert_eq!(message.parts().len(), 1);
    assert!(message.headers().is_empty());

    assert!(MessageParser::new().parse(b"").is_none());
    assert!(MessageParser::new().parse(b"hello").is_none());
    assert!(
        MessageParser::new()
            .parse(b"hello world\nfoo bar\n")
            .is_none()
    );
}

#[test]
fn multipart_structure() {
    for raw in [MULTIPART.to_vec(), with_crlf(MULTIPART)] {
        let message = parse(&raw);
        let root = message.root_part();
        assert!(matches!(root.kind(), PartKind::Multipart));
        assert_eq!(root.children().map(|p| p.id()).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(root.offset_end(), raw.len() as u32);
        assert_eq!(root.boundary(), Some("b"));
        assert_eq!(text_of(&message, 1), "hello");
        assert_eq!(text_of(&message, 2), "<p>x</p>");
        let second = message.part(2).expect("part");
        assert!(matches!(second.kind(), PartKind::Html));
        assert_eq!(second.delimiter(), Some("b"));
        assert_eq!(second.parent().map(|p| p.id()), Some(0));
        assert!(message.parts().all(|part| part.flags().is_empty()));
        assert_eq!(message.part_by_boundary("b").map(|p| p.id()), Some(0));
        assert_eq!(
            second.raw(),
            if raw.contains(&b'\r') {
                &b"Content-Type: text/html\r\n\r\n<p>x</p>"[..]
            } else {
                &b"Content-Type: text/html\n\n<p>x</p>"[..]
            }
        );
    }
}

#[test]
fn delimiter_rule() {
    let raw = regression!("delimiter-rule");
    let message = parse(raw);
    assert_eq!(
        text_of(&message, 1),
        "visit --BND for details\n--BNDX not a delimiter"
    );
    assert_eq!(text_of(&message, 2), "second");
    assert!(message.parts().all(|part| part.flags().is_empty()));
}

#[test]
fn fallback_and_missing_delimiters() {
    let raw = regression!("fallback-delimiter-mid-line");
    let message = parse(raw);
    let part = message.part(1).expect("part");
    assert_eq!(text_of(&message, 1), "text invalid");
    assert!(part.flags().contains(PartFlags::FALLBACK_DELIMITER));
    assert!(message.root_part().flags().is_empty());

    let raw = regression!("missing-close-delimiter");
    let message = parse(raw);
    let part = message.part(1).expect("part");
    assert!(part.flags().contains(PartFlags::MISSING_DELIMITER));
    assert_eq!(text_of(&message, 1), "no end\n");
    assert!(
        message
            .root_part()
            .flags()
            .contains(PartFlags::UNTERMINATED)
    );

    let raw = regression!("part-without-blank-line");
    let message = parse(raw);
    let first = message.part(1).expect("part");
    assert!(first.flags().contains(PartFlags::NO_BLANK_LINE));
    assert_eq!(first.raw_body(), b"");
    assert_eq!(text_of(&message, 2), "ok");
}

#[test]
fn inner_multipart_closed_by_outer_delimiter() {
    let raw = regression!("inner-multipart-closed-by-outer");
    let message = parse(raw);
    assert_eq!(message.parts().len(), 4);
    assert!(
        message
            .part(1)
            .expect("inner")
            .flags()
            .contains(PartFlags::UNTERMINATED)
    );
    assert_eq!(text_of(&message, 2), "a");
    assert_eq!(text_of(&message, 3), "b");
    assert_eq!(
        message.part(3).and_then(|p| p.parent()).map(|p| p.id()),
        Some(0)
    );
}

#[test]
fn nested_inline_message() {
    for raw in [NESTED_INLINE.to_vec(), with_crlf(NESTED_INLINE)] {
        let message = parse(&raw);
        assert_eq!(message.messages().len(), 2);
        assert_eq!(message.parts().len(), 4);
        let container = message.part(2).expect("container");
        let nested = container.nested().expect("nested");
        assert_eq!(nested.subject(), Some("nested"));
        assert_eq!(nested.root_part().id(), 3);
        assert_eq!(nested.container().map(|p| p.id()), Some(2));
        assert_eq!(nested.source(), Source::Raw);
        assert_eq!(nested.root_part().offset_header(), container.offset_body());
        assert_eq!(text_of(&message, 3), "nested body");
        assert_eq!(nested.body_text(0).as_deref(), Some("nested body"));
        assert!(container.is_attachment());
        assert_eq!(
            message.attachments().map(|p| p.id()).collect::<Vec<_>>(),
            [2]
        );
        assert_eq!(
            message
                .root_part()
                .children()
                .map(|p| p.id())
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(container.offset_end(), nested.root_part().offset_end());
    }
}

#[test]
fn nested_encoded_message() {
    for raw in [NESTED_ENCODED.to_vec(), with_crlf(NESTED_ENCODED)] {
        let message = parse(&raw);
        assert_eq!(message.messages().len(), 2);
        assert_eq!(message.parts().len(), 4);
        let container = message.part(1).expect("container");
        assert!(matches!(container.kind(), PartKind::Message(_)));
        let nested = container.nested().expect("nested");
        assert_eq!(nested.subject(), Some("inner"));
        assert_eq!(nested.source(), Source::Decoded(1));
        assert_eq!(nested.root_part().id(), 2);
        assert_eq!(nested.root_part().offset_header(), 0);
        assert_eq!(
            nested.root_part().offset_end() as usize,
            nested.source_bytes().len()
        );
        assert_eq!(text_of(&message, 2), "inner body\n");
        assert_eq!(text_of(&message, 3), "after");
        assert_eq!(
            message.part(3).and_then(|p| p.parent()).map(|p| p.id()),
            Some(0)
        );
        assert_eq!(
            message
                .root_part()
                .children()
                .map(|p| p.id())
                .collect::<Vec<_>>(),
            [1, 3]
        );
        assert_eq!(container.decoded().as_ref(), nested.source_bytes());
    }
}

fn wrap_encoded(inner: &str, levels: usize) -> String {
    let mut message = inner.to_string();
    for level in 0..levels {
        message = format!(
            "Subject: level {level}\nContent-Type: message/rfc822\nContent-Transfer-Encoding: base64\n\n{}\n",
            base64::MIME.encode(&message)
        );
    }
    message
}

#[test]
fn encoded_inside_encoded_and_limits() {
    let raw = wrap_encoded("Subject: core\n\ncore body\n", 2);
    let message = parse(raw.as_bytes());
    assert_eq!(message.messages().len(), 3);
    assert_eq!(message.parts().len(), 3);
    let innermost = message.messages().last().expect("innermost");
    assert_eq!(innermost.subject(), Some("core"));
    assert_eq!(innermost.body_text(0).as_deref(), Some("core body\n"));
    assert_eq!(innermost.source(), Source::Decoded(1));

    let raw = wrap_encoded("Subject: core\n\ncore body\n", 4);
    let message = parse(raw.as_bytes());
    assert_eq!(message.messages().len(), 4);
    let last = message.parts().last().expect("part");
    assert!(matches!(last.kind(), PartKind::Binary));
    assert!(last.flags().contains(PartFlags::NESTING_LIMIT));

    let message = MessageParser::new()
        .max_encoded_nesting(1)
        .parse(raw.as_bytes())
        .expect("parses");
    assert_eq!(message.messages().len(), 2);
    assert!(
        message
            .part(1)
            .expect("part")
            .flags()
            .contains(PartFlags::NESTING_LIMIT)
    );

    let message = MessageParser::new()
        .max_encoded_nesting(0)
        .parse(raw.as_bytes())
        .expect("parses");
    assert_eq!(message.messages().len(), 1);
    let root = message.root_part();
    assert!(matches!(root.kind(), PartKind::Binary));
    assert!(root.flags().is_empty());
}

#[test]
fn empty_nested_messages() {
    let raw = regression!("empty-nested-message-in-multipart");
    let message = parse(raw);
    assert_eq!(message.messages().len(), 1);
    let part = message.part(1).expect("part");
    assert!(part.flags().contains(PartFlags::NO_BLANK_LINE));
    assert!(part.nested().is_none());

    let message = parse(regression!("encoded-message-without-header"));
    assert_eq!(message.messages().len(), 1);
    let root = message.root_part();
    assert!(matches!(root.kind(), PartKind::Binary));
    assert!(root.flags().contains(PartFlags::NO_BLANK_LINE));
    assert_eq!(root.decoded().as_ref(), b"no headers at all");
}

#[test]
fn depth_and_part_limits() {
    let raw = regression!("nested-multipart-depth");
    let message = MessageParser::new()
        .max_depth(2)
        .parse(raw)
        .expect("parses");
    assert_eq!(message.parts().len(), 2);
    let inner = message.part(1).expect("inner");
    assert!(inner.flags().contains(PartFlags::LIMIT_REACHED));
    assert!(matches!(inner.kind(), PartKind::Text));

    let message = MessageParser::new()
        .max_parts(2)
        .parse(MULTIPART)
        .expect("parses");
    assert_eq!(message.parts().len(), 2);
    let last = message.part(1).expect("last");
    assert!(last.flags().contains(PartFlags::LIMIT_REACHED));
    assert_eq!(last.offset_end() as usize, MULTIPART.len());

    let mut deep = String::new();
    for level in 0..200 {
        deep.push_str(&format!(
            "Content-Type: multipart/mixed; boundary=\"b{level}\"\n\n--b{level}\n"
        ));
    }
    let message = parse(deep.as_bytes());
    assert!(message.parts().len() <= 64);
    assert!(
        message
            .parts()
            .any(|part| part.flags().contains(PartFlags::LIMIT_REACHED))
    );
}

#[test]
fn headers_only() {
    let raw = MULTIPART;
    let message = MessageParser::new().parse_headers(raw).expect("parses");
    let full = parse(raw);
    assert_eq!(message.parts().len(), 1);
    let root = message.root_part();
    assert_eq!(root.offset_body(), full.root_part().offset_body());
    assert_eq!(root.offset_end() as usize, raw.len());
    assert_eq!(message.headers().len(), full.headers().len());
    assert_eq!(message.text_body().len(), 0);
}

#[test]
fn buffer_reuse_and_ownership() {
    let parser = MessageParser::new();
    let mut buffers = MessageBuffers::new();
    for raw in [
        MULTIPART,
        b"",
        NESTED_INLINE,
        regression!("subject-and-body"),
    ] {
        let Some(reused) = parser.parse_with(raw, &mut buffers) else {
            assert!(parser.parse(raw).is_none());
            assert_eq!(buffers.data.headers.capacity(), 0);
            continue;
        };
        assert_eq!(dump(&reused), dump(&parse(raw)));
        assert_eq!(buffers.data.headers.capacity(), 0);
        buffers = reused.into_buffers();
        assert!(buffers.data.parts.is_empty() && buffers.data.headers.capacity() > 0);
    }
    let owned = parser.parse_owned(MULTIPART.to_vec()).expect("parses");
    assert_eq!(dump(&owned), dump(&parse(MULTIPART)));
    let copied: Message<'static> = parse(MULTIPART).into_owned();
    assert_eq!(dump(&copied), dump(&owned));
}

#[test]
fn unknown_headers_apply_to_vendor_names() {
    let raw = regression!("vendor-encoded-words");
    let values = |parser: MessageParser| {
        let message = parser.parse(raw).expect("parses");
        message
            .headers()
            .iter()
            .map(|header| header.value().as_text().unwrap_or_default().to_string())
            .collect::<Vec<_>>()
    };
    let raw_values = [
        "=?utf-8?q?caf=C3=A9?=",
        "=?utf-8?q?1?=",
        "=?utf-8?q?a?=",
        "=?utf-8?q?o?=",
        "s",
    ];
    assert_eq!(values(MessageParser::new()), raw_values);
    assert_eq!(
        values(MessageParser::new().unknown_headers(HeaderForm::Text)),
        ["caf\u{e9}", "1", "a", "=?utf-8?q?o?=", "s"]
    );
    for parser in [
        MessageParser::new()
            .header(HeaderName::XMailer, HeaderForm::Raw)
            .unknown_headers(HeaderForm::Text),
        MessageParser::new()
            .unknown_headers(HeaderForm::Text)
            .header(HeaderName::XMailer, HeaderForm::Raw),
    ] {
        assert_eq!(
            values(parser),
            ["=?utf-8?q?caf=C3=A9?=", "1", "a", "=?utf-8?q?o?=", "s"]
        );
    }
    assert_eq!(
        values(
            MessageParser::new()
                .unknown_headers(HeaderForm::Text)
                .unknown_headers(HeaderForm::Raw)
        ),
        raw_values
    );
    assert_eq!(
        MessageParser::new().unknown_headers(HeaderForm::Raw),
        MessageParser::new()
    );
}

#[test]
fn message_is_send_sync_and_covariant() {
    fn send_sync<T: Send + Sync + Clone + Default>() {}
    send_sync::<Message<'static>>();
    fn covariant<'a>(message: Message<'static>) -> Message<'a> {
        message
    }
    let message = covariant(parse(MULTIPART).into_owned());
    assert_eq!(message.parts().len(), 3);
}

#[test]
fn fixtures_parse_with_every_kernel() {
    let files = fixture_files();
    assert!(files.len() > 100);
    for file in files {
        for raw in [with_lf(&file), with_crlf(&file)] {
            let reference = MessageParser::new()
                .kernel(Kernel::SCALAR)
                .parse(&raw)
                .map(|message| dump(&message));
            for kernel in Kernel::available() {
                let parsed = MessageParser::new()
                    .kernel(kernel)
                    .parse(&raw)
                    .map(|message| dump(&message));
                assert_eq!(parsed, reference, "{} differs", kernel.name());
            }
        }
    }
}

#[test]
fn truncated_inputs_never_panic() {
    for file in fixture_files() {
        let raw = with_crlf(&file);
        for end in (0..raw.len()).step_by(7) {
            if let Some(message) = MessageParser::new().parse(raw.get(..end).unwrap_or_default()) {
                let _ = dump(&message);
            }
        }
    }
}

#[test]
fn decoding_accessors() {
    let raw = regression!("decoding-accessors");
    let message = parse(raw);
    let text = message.part(1).expect("text");
    assert_eq!(text.encoding(), crate::Encoding::QuotedPrintable);
    assert_eq!(text.decoded().as_ref(), b"caf\xe9 soft break");
    assert_eq!(text.decoded_len(), 15);
    assert_eq!(text.text().as_deref(), Some("café soft break"));
    let mut out = String::from(">");
    assert!(text.text_into(&mut out));
    assert_eq!(out, ">café soft break");
    let binary = message.part(2).expect("binary");
    assert!(matches!(binary.kind(), PartKind::Binary));
    assert_eq!(binary.decoded().as_ref(), b"Hello");
    assert_eq!(binary.decoded_len(), 5);
    assert!(binary.text().is_none());
    let mut bytes = b"<".to_vec();
    binary.decode_into(&mut bytes);
    assert_eq!(bytes, b"<Hello");
    assert!(!binary.text_into(&mut out));
    assert_eq!(message.root_part().children().len(), 2);
}

#[test]
fn header_views() {
    let raw = regression!("header-views");
    let message = parse(raw);
    let headers = message.headers();
    assert_eq!(headers.len(), 4);
    let odd = headers.iter().nth(1).expect("header");
    assert_eq!(odd.raw_name(), "X-Te\u{fffd}st");
    assert!(odd.name().is_other());
    assert_eq!(odd.value().as_text(), Some("value"));
    let subject = headers.get(HeaderName::Subject).expect("subject");
    assert_eq!(subject.raw_value(), b" hello\n");
    let reparsed = subject.parse_as(crate::HeaderForm::ContentType);
    assert_eq!(
        reparsed.value().as_content_type().map(|ct| ct.ctype()),
        Some("hello")
    );
    assert!(headers.contains("reply-to"));
    assert!(headers.has_known());
    assert_eq!(headers.all(HeaderName::Subject).count(), 1);
    assert_eq!(headers.all("x-te\u{fffd}st").count(), 1);
}

#[test]
fn decode_problems() {
    let raw = regression!("decode-problems");
    let message = parse(raw);
    let shift_jis = if cfg!(feature = "full_encoding") {
        ("\u{30cf}", DecodeProblems::default())
    } else {
        (
            "\u{fffd}n",
            DecodeProblems::UNKNOWN_CHARSET | DecodeProblems::MALFORMED_CHARSET,
        )
    };
    let expected = [
        ("caf\u{e9}", DecodeProblems::default()),
        ("caf\u{e9}", DecodeProblems::MALFORMED_TRANSFER_ENCODING),
        ("caf\u{e9} =ZZ", DecodeProblems::MALFORMED_TRANSFER_ENCODING),
        ("caf\u{e9}", DecodeProblems::UNKNOWN_CHARSET),
        ("caf\u{fffd}", DecodeProblems::MALFORMED_CHARSET),
        ("", DecodeProblems::UNKNOWN_TRANSFER_ENCODING),
        ("\u{fffd}", DecodeProblems::MALFORMED_CHARSET),
        shift_jis,
    ];
    assert_eq!(message.parts().len(), expected.len() + 1);
    for (part, (text, problems)) in message.parts().skip(1).zip(expected) {
        let (decoded, transfer) = part.decoded_checked();
        assert_eq!(decoded, part.decoded(), "part {}", part.id());
        let checked = part.text_checked();
        assert_eq!(checked.clone().map(|(text, _)| text), part.text());
        match checked {
            Some((checked, found)) => {
                assert_eq!(
                    (checked.as_ref(), found),
                    (text, problems),
                    "part {}",
                    part.id()
                );
                assert!(found.contains(transfer), "part {}", part.id());
            }
            None => assert_eq!(transfer, problems, "part {}", part.id()),
        }
    }
    assert_eq!(
        DecodeProblems::MALFORMED_TRANSFER_ENCODING
            .names()
            .collect::<Vec<_>>(),
        ["malformed_transfer_encoding"]
    );
}

#[test]
fn checked_decoding_matches_the_fast_path_on_fixtures() {
    for file in fixture_files() {
        for raw in [with_lf(&file), with_crlf(&file)] {
            let Some(message) = MessageParser::new().parse(&raw) else {
                continue;
            };
            for part in message.parts() {
                assert_eq!(part.decoded_checked().0, part.decoded());
                assert_eq!(part.text_checked().map(|(text, _)| text), part.text());
            }
        }
    }
}

fn spelled_name(raw: &[u8], offset_field: u32, offset_start: u32) -> String {
    let region = raw
        .get(offset_field as usize..(offset_start as usize).saturating_sub(1))
        .unwrap_or_default();
    String::from_utf8_lossy(crate::header_name::trim_blank_end(region)).into_owned()
}

#[test]
fn raw_names_keep_the_original_spelling() {
    let mut raw = Vec::new();
    let mut known = Vec::new();
    for id in 0..crate::header_name::KNOWN_NAMES as u16 {
        let name = crate::header_name::canonical(id);
        let (head, tail) = name.split_at(name.len() / 2);
        for (spelling, is_known) in [
            (name.to_string(), true),
            (name.to_ascii_lowercase(), true),
            (name.to_ascii_uppercase(), true),
            (format!(":{name}"), false),
            (format!("{name} \t"), true),
            (format!("{head} {tail}"), false),
        ] {
            raw.extend_from_slice(spelling.as_bytes());
            raw.extend_from_slice(b": value\r\n");
            known.push(is_known);
        }
    }
    raw.extend_from_slice(b"X-Other: v\nx-caf\xc3\xa9 : v\nX-\xff: v\n\nbody");
    known.extend([false; 3]);
    let message = parse(&raw);
    let headers = message.headers();
    assert_eq!(headers.len(), known.len());
    for ((index, header), is_known) in headers.iter().enumerate().zip(known) {
        assert_eq!(
            header.raw_name(),
            spelled_name(&raw, header.offset_field(), header.offset_start()),
            "header {index}"
        );
        assert_eq!(header.name().is_other(), !is_known, "header {index}");
        if is_known {
            assert!(
                header
                    .raw_name()
                    .eq_ignore_ascii_case(header.name().as_str())
            );
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn serialize_message() {
    let message = parse(NESTED_INLINE);
    let json = serde_json::to_value(&message).expect("serializes");
    assert_eq!(json["messages"].as_array().map(Vec::len), Some(2));
    assert_eq!(json["messages"][1]["container"], 2);
    assert_eq!(json["parts"][0]["kind"], "multipart");
    assert_eq!(json["parts"][0]["children"], serde_json::json!([1, 2]));
    assert_eq!(json["parts"][2]["nested"], 1);
    assert_eq!(json["parts"][3]["text"], "nested body");
    assert_eq!(
        json["parts"][0]["headers"][0]["value"]["content_type"]["attributes"][0],
        serde_json::json!(["boundary", "outer"])
    );
    let message = parse(NESTED_ENCODED);
    let json = serde_json::to_value(&message).expect("serializes");
    assert_eq!(json["messages"][1]["source"]["decoded"], 1);
    let binary = MessageParser::new()
        .max_encoded_nesting(0)
        .parse(NESTED_ENCODED)
        .expect("parses");
    let json = serde_json::to_value(&binary).expect("serializes");
    assert_eq!(json["parts"][1]["kind"], "binary");
    assert!(
        json["parts"][1]["binary"]
            .as_str()
            .is_some_and(|b64| !b64.is_empty())
    );
}

#[test]
fn mutated_inputs_never_panic() {
    const TOKENS: [&[u8]; 12] = [
        b"\n",
        b"\r\n",
        b"\n--",
        b"--b",
        b"--outer--",
        b":",
        b"=?utf-8?q?a?=",
        b"\n\n",
        b" ",
        b"Content-Type: message/rfc822\n",
        b"Content-Transfer-Encoding: base64\n",
        b"\"",
    ];
    let mut rng = crate::scan::tests::Rng(0x0123_4567_89ab_cdef);
    let seeds = [MULTIPART, NESTED_INLINE, NESTED_ENCODED];
    for seed in seeds {
        for _ in 0..1500 {
            let mut raw = seed.to_vec();
            for _ in 0..1 + rng.below(4) {
                let at = rng.below(raw.len() + 1);
                match rng.below(3) {
                    0 => {
                        let token = TOKENS
                            .get(rng.below(TOKENS.len()))
                            .copied()
                            .unwrap_or_default();
                        raw.splice(at..at, token.iter().copied());
                    }
                    1 => {
                        let end = (at + rng.below(16)).min(raw.len());
                        raw.drain(at..end);
                    }
                    _ => {
                        if let Some(byte) = raw.get_mut(at) {
                            *byte = rng.next() as u8;
                        }
                    }
                }
            }
            let [scalar, best] = [Kernel::SCALAR, Kernel::best()].map(|kernel| {
                MessageParser::new()
                    .kernel(kernel)
                    .parse(&raw)
                    .map(|message| dump(&message))
            });
            assert_eq!(scalar, best, "{raw:?}");
        }
    }
}
