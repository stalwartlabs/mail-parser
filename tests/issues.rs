/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod support;

use mail_parser::{
    Address, Message, MessageParser, PartFlags, PartKind, PartRole, html_to_text, text_to_html,
};
use std::path::PathBuf;
use support::{fixture, ids, line_endings, lists, regression, resource};

fn texts(raw: &[u8]) -> Vec<(String, PartFlags)> {
    MessageParser::new()
        .parse(raw)
        .expect("the message parses")
        .parts()
        .filter(|part| matches!(part.kind(), PartKind::Text))
        .map(|part| (part.text().unwrap_or_default().into_owned(), part.flags()))
        .collect()
}

#[test]
fn issue_156_mid_line_boundary() {
    for (raw, eol) in line_endings(&fixture("156.eml")) {
        let message = MessageParser::new()
            .parse(&raw)
            .expect("the message parses");
        assert_eq!(message.parts().len(), 2);
        assert!(matches!(message.root_part().kind(), PartKind::Multipart));
        assert_eq!(
            texts(&raw),
            [(
                format!("visit --BND for details{eol}SECRET"),
                PartFlags::default()
            )]
        );
        assert!(message.parts().all(|part| !part.has_problems()));
        assert_eq!(
            message
                .text_body()
                .map(|part| part.id())
                .collect::<Vec<_>>(),
            [1]
        );
    }
}

#[test]
fn issue_156_boundary_with_suffix() {
    for (raw, eol) in line_endings(&fixture("156-suffix.eml")) {
        assert_eq!(
            texts(&raw),
            [(
                format!("first{eol}--BNDX not a delimiter{eol}last"),
                PartFlags::default()
            )]
        );
    }
}

#[test]
fn issue_156_fallback_is_flagged() {
    for (raw, _) in line_endings(&fixture("156-fallback.eml")) {
        let parts = texts(&raw);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], ("first part".to_string(), PartFlags::default()));
        assert_eq!(parts[1].0, "second part ends here");
        assert!(parts[1].1.contains(PartFlags::FALLBACK_DELIMITER));
        let message = MessageParser::new()
            .parse(&raw)
            .expect("the message parses");
        assert!(
            !message
                .root_part()
                .flags()
                .contains(PartFlags::UNTERMINATED)
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Item<'x> {
    Mailbox(&'x str),
    Group(&'x str, Vec<&'x str>),
}

#[test]
fn issue_98_address_lists_in_document_order() {
    let mailbox = Item::Mailbox;
    let cases = [
        ("98-list.eml", vec![mailbox("x@y.com"), mailbox("x@y.com")]),
        (
            "98-mixed.eml",
            vec![
                mailbox("x@y.com"),
                Item::Group("group", vec!["some text"]),
                mailbox("x@y.com"),
            ],
        ),
        (
            "98-group-first.eml",
            vec![
                Item::Group("first", vec!["a@b.c", "d@e.f"]),
                mailbox("x@y.com"),
            ],
        ),
        (
            "98-group-last.eml",
            vec![mailbox("x@y.com"), Item::Group("last", vec!["a@b.c"])],
        ),
        (
            "98-adjacent-groups.eml",
            vec![
                Item::Group("one", vec!["a@b.c"]),
                Item::Group("two", vec!["d@e.f"]),
            ],
        ),
    ];
    for (name, expected) in cases {
        for (raw, _) in line_endings(&fixture(name)) {
            let message = MessageParser::new()
                .parse(&raw)
                .expect("the message parses");
            let to = message.to().expect("To is an address list");
            let items: Vec<_> = to
                .iter()
                .map(|item| match item {
                    Address::Mailbox(mailbox) => {
                        Item::Mailbox(mailbox.address().unwrap_or_default())
                    }
                    Address::Group(group) => Item::Group(
                        group.name().unwrap_or_default(),
                        group
                            .mailboxes()
                            .map(|member| member.address().or(member.name()).unwrap_or_default())
                            .collect(),
                    ),
                })
                .collect();
            assert_eq!(items, expected, "{name}");
            assert_eq!(
                to.has_groups(),
                expected.iter().any(|item| matches!(item, Item::Group(..))),
                "{name}"
            );
        }
    }
}

#[test]
fn every_fixture_parses_in_both_line_endings() {
    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/eml")];
    let mut count = 0;
    while let Some(dir) = dirs.pop() {
        for path in std::fs::read_dir(&dir)
            .expect("directory")
            .flatten()
            .map(|entry| entry.path())
        {
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "eml") {
                let raw = std::fs::read(&path).expect("file");
                for (raw, _) in line_endings(&raw) {
                    if let Some(message) = MessageParser::new().parse(&raw) {
                        for part in message.parts() {
                            let _ = part.decoded();
                            let _ = part.text();
                            for header in part.headers().iter() {
                                let _ = header.value();
                                let _ = header.name();
                            }
                        }
                    }
                }
                count += 1;
            }
        }
    }
    assert!(count > 100);
}

#[test]
fn issue_70_inline_images_in_related() {
    for (raw, eol) in line_endings(&fixture("70.eml")) {
        let message = MessageParser::new()
            .parse(&raw)
            .expect("the message parses");
        assert_eq!(message.parts().len(), 6);
        assert_eq!(lists(message.root()), [vec![2], vec![3], vec![4, 5]]);
        for (id, body) in [(4, "A"), (5, "B")] {
            let image = message.part(id).expect("image part");
            assert!(image.is_inline(), "part {id}");
            assert!(matches!(image.kind(), PartKind::InlineBinary), "part {id}");
            assert_eq!(image.decoded().as_ref(), format!("{body}{eol}").as_bytes());
            assert_eq!(image.role(), PartRole::Attachment);
        }
        for id in [0, 1, 2, 3] {
            assert!(!message.part(id).expect("part").is_inline(), "part {id}");
        }
        assert!(!message.has_attachments());
    }
}

#[test]
fn issue_67_legacy_032_lists() {
    for (raw, _) in line_endings(&resource("legacy/032.eml")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        assert_eq!(
            lists(message.root()),
            [vec![2, 4, 5], vec![3, 4, 5], vec![]]
        );
        assert_eq!(message.other_parts().count(), 0);
        assert!(!message.has_attachments());
    }
}

#[test]
fn issue_67_alternative_with_inline_image() {
    for (raw, _) in line_endings(&fixture("67-alternative.eml")) {
        let message = MessageParser::new()
            .parse(&raw)
            .expect("the message parses");
        assert_eq!(lists(message.root()), [vec![1], vec![1], vec![2]]);
        let image = message.part(2).expect("image");
        assert!(image.is_inline());
        assert!(matches!(image.kind(), PartKind::InlineBinary));
        assert_eq!(image.role(), PartRole::Attachment);
        assert_eq!(message.part(1).expect("text").role(), PartRole::CopiedBody);
        assert!(!message.has_attachments());
    }
}

fn assert_partition(message: &Message<'_>, expected: &[PartRole]) {
    assert_eq!(
        message.parts().map(|part| part.role()).collect::<Vec<_>>(),
        expected
    );
    for part in message.parts() {
        let owner = part.message();
        let in_text = owner.text_body().any(|item| item.id() == part.id());
        let in_html = owner.html_body().any(|item| item.id() == part.id());
        let in_attachments = owner.attachments().any(|item| item.id() == part.id());
        let buckets = [
            part.role() == PartRole::Container,
            matches!(
                part.role(),
                PartRole::TextBody
                    | PartRole::HtmlBody
                    | PartRole::CopiedBody
                    | PartRole::MediaBody
            ) && (in_text || in_html),
            part.role() == PartRole::Attachment && in_attachments && !in_text && !in_html,
            part.role() == PartRole::Dropped && !in_text && !in_html && !in_attachments,
        ];
        assert_eq!(
            buckets.iter().filter(|bucket| **bucket).count(),
            1,
            "part {} is in exactly one role",
            part.id()
        );
    }
}

#[test]
fn issue_107_every_part_has_one_role() {
    use PartRole::{Container, HtmlBody, MediaBody, TextBody};
    for (raw, _) in line_endings(&resource("legacy/032.eml")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        assert_partition(
            &message,
            &[
                Container, Container, TextBody, HtmlBody, MediaBody, MediaBody,
            ],
        );
        assert_eq!(message.other_parts().count(), 0);
    }
    for (raw, _) in line_endings(&fixture("107-image-first.eml")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        assert_partition(
            &message,
            &[
                Container, MediaBody, Container, TextBody, HtmlBody, MediaBody,
            ],
        );
        assert_eq!(
            lists(message.root()),
            [vec![1, 3, 5], vec![1, 4, 5], vec![]]
        );
        assert!(message.body_html(0).is_none());
        assert_eq!(
            message
                .parts()
                .filter(|part| part.role() == HtmlBody)
                .map(|part| part.id())
                .collect::<Vec<_>>(),
            [4]
        );
    }
}

#[test]
fn issue_73_text_bodies_yield_text() {
    for (raw, _) in line_endings(&fixture("73.eml")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        let html_part = message.part(1).expect("html part");
        assert!(matches!(html_part.kind(), PartKind::Html));
        assert_eq!(ids(message.text_body()), [1]);
        assert_eq!(ids(message.html_body()), [1]);
        assert_eq!(html_part.role(), PartRole::CopiedBody);

        let html = html_part.text().expect("html text");
        assert!(html.starts_with("<html><p>I was thinking"));
        let texts: Vec<String> = message
            .text_bodies()
            .map(|text| text.into_owned())
            .collect();
        assert_eq!(texts, [html_to_text(&html)]);
        assert!(!texts[0].contains('<'));
        assert!(texts[0].contains("the \u{201c}exporting\u{201d} to focus"));
        assert_eq!(message.body_text(0).as_deref(), Some(texts[0].as_str()));

        let htmls: Vec<String> = message
            .html_bodies()
            .map(|html| html.into_owned())
            .collect();
        assert_eq!(htmls, [html.as_ref()]);
    }

    for (raw, _) in line_endings(&regression("mixed-text-image-html")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        assert_eq!(ids(message.text_body()), [1, 2, 3]);
        let texts: Vec<String> = message
            .text_bodies()
            .map(|text| text.into_owned())
            .collect();
        assert_eq!(texts, ["first".to_string(), html_to_text("<p>third</p>")]);
        let htmls: Vec<String> = message
            .html_bodies()
            .map(|html| html.into_owned())
            .collect();
        assert_eq!(htmls, [text_to_html("first"), "<p>third</p>".to_string()]);
    }
}

const OUTER_HEADERS: &str = "From: Boundary Tester <tester@example.com>\n\
To: Recipient <recipient@example.com>\n\
Subject: Boundaries of every part\n\
MIME-Version: 1.0\n\
Content-Type: multipart/mixed; boundary=\"outer-mixed\"\n\n";
const ALTERNATIVE_HEADERS: &str =
    "Content-Type: multipart/alternative; boundary=\"inner-alternative\"\n\n";
const PLAIN_HEADERS: &str = "Content-Type: text/plain; charset=us-ascii\n\n";
const HTML_HEADERS: &str = "Content-Type: text/html; charset=us-ascii\n\n";
const RFC822_HEADERS: &str = "Content-Type: message/rfc822\nContent-Disposition: inline\n\n";
const NESTED_HEADERS: &str = "From: Nested Sender <nested@example.com>\n\
Subject: Nested message\n\
MIME-Version: 1.0\n\
Content-Type: multipart/mixed; boundary=\"nested-mixed\"\n\n";
const BINARY_HEADERS: &str =
    "Content-Type: application/octet-stream\nContent-Transfer-Encoding: base64\n\n";
const ALTERNATIVE_BODY: &str = "--inner-alternative\n\
Content-Type: text/plain; charset=us-ascii\n\n\
Plain version.\n\
--inner-alternative\n\
Content-Type: text/html; charset=us-ascii\n\n\
<p>HTML version.</p>\n\
--inner-alternative--\n\
Alternative epilogue.";
const NESTED_BODY: &str = "--nested-mixed\n\
Content-Type: text/plain; charset=us-ascii\n\n\
Nested text.\n\
--nested-mixed\n\
Content-Type: application/octet-stream\n\
Content-Transfer-Encoding: base64\n\n\
AAECAw==\n\
--nested-mixed--";

#[test]
fn issue_127_boundaries_and_raw_ranges() {
    let original = fixture("127.eml");
    let original = String::from_utf8(original).expect("ascii fixture");
    let nested = format!("{NESTED_HEADERS}{NESTED_BODY}");
    let expected: [(Option<&str>, Option<&str>, &str, String); 8] = [
        (
            Some("outer-mixed"),
            None,
            OUTER_HEADERS,
            original
                .strip_prefix(OUTER_HEADERS)
                .expect("headers first")
                .to_string(),
        ),
        (
            Some("inner-alternative"),
            Some("outer-mixed"),
            ALTERNATIVE_HEADERS,
            ALTERNATIVE_BODY.to_string(),
        ),
        (
            None,
            Some("inner-alternative"),
            PLAIN_HEADERS,
            "Plain version.".to_string(),
        ),
        (
            None,
            Some("inner-alternative"),
            HTML_HEADERS,
            "<p>HTML version.</p>".to_string(),
        ),
        (None, Some("outer-mixed"), RFC822_HEADERS, nested.clone()),
        (
            Some("nested-mixed"),
            None,
            NESTED_HEADERS,
            NESTED_BODY.to_string(),
        ),
        (
            None,
            Some("nested-mixed"),
            PLAIN_HEADERS,
            "Nested text.".to_string(),
        ),
        (
            None,
            Some("nested-mixed"),
            BINARY_HEADERS,
            "AAECAw==".to_string(),
        ),
    ];
    for (raw, eol) in line_endings(original.as_bytes()) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        assert_eq!(message.parts().len(), expected.len());
        assert_eq!(message.messages().len(), 2);
        for (part, (boundary, delimiter, headers, body)) in message.parts().zip(&expected) {
            let headers = headers.replace('\n', eol);
            let body = body.replace('\n', eol);
            let id = part.id();
            assert_eq!(part.boundary(), *boundary, "part {id} boundary");
            assert_eq!(part.delimiter(), *delimiter, "part {id} delimiter");
            assert_eq!(part.raw_headers(), headers.as_bytes(), "part {id} headers");
            assert_eq!(part.raw_body(), body.as_bytes(), "part {id} body");
            assert_eq!(
                part.raw(),
                format!("{headers}{body}").as_bytes(),
                "part {id} raw"
            );
        }
        assert_eq!(message.root_part().raw(), raw.as_slice());
        for (boundary, id) in [
            ("outer-mixed", 0),
            ("inner-alternative", 1),
            ("nested-mixed", 5),
        ] {
            assert_eq!(
                message.part_by_boundary(boundary).map(|part| part.id()),
                Some(id),
                "{boundary}"
            );
        }
        assert!(message.part_by_boundary("missing").is_none());
        assert!(message.part_by_boundary("outer").is_none());
        let nested_message = message
            .part(4)
            .and_then(|part| part.nested())
            .expect("nested");
        assert_eq!(nested_message.root_part().id(), 5);
        assert_eq!(nested_message.raw(), nested.replace('\n', eol).as_bytes());
        assert!(message.part(4).expect("rfc822").is_inline());
    }
}
