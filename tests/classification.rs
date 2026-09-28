/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod support;

use mail_parser::{Message, MessageParser, PartKind, PartRole};
use std::path::PathBuf;
use support::{ids, line_endings, lists, resource};

#[test]
fn rfc8621_example() {
    for (raw, _) in line_endings(&resource("rfc/007.eml")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        assert_eq!(
            lists(message.root()),
            [
                vec![1, 5, 6, 7, 15],
                vec![1, 9, 15],
                vec![6, 10, 11, 12, 13]
            ]
        );
        let nested = message.messages().nth(1).expect("nested message J");
        assert_eq!(lists(nested), [vec![14], vec![14], vec![]]);
    }
}

#[test]
fn has_attachments_is_per_message() {
    for (raw, _) in line_endings(&support::fixture("127.eml")) {
        let message = MessageParser::new().parse(&raw).expect("parses");
        let flags: Vec<bool> = message
            .messages()
            .map(|owner| owner.has_attachments())
            .collect();
        assert_eq!(flags, [false, true]);
    }
}

const DROPPED: [&str; 2] = ["alternative-dropped-footer", "alternative-nested-dropped"];

fn fixtures() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/eml")];
    let mut files = Vec::new();
    while let Some(dir) = dirs.pop() {
        for path in std::fs::read_dir(&dir)
            .expect("directory")
            .flatten()
            .map(|entry| entry.path())
        {
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "eml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn check_roles(message: &Message<'_>, name: &str) -> usize {
    for part in message.parts() {
        let owner = part.message();
        let in_text = owner.text_body().any(|item| item.id() == part.id());
        let in_html = owner.html_body().any(|item| item.id() == part.id());
        let in_attachments = owner.attachments().any(|item| item.id() == part.id());
        let context = format!("{name} part {} {:?}", part.id(), part.role());
        let kind = part.kind();
        let listed = in_text || in_html;
        let consistent = match part.role() {
            PartRole::Container => {
                matches!(kind, PartKind::Multipart) && !listed && !in_attachments
            }
            PartRole::TextBody => matches!(kind, PartKind::Text) && listed,
            PartRole::HtmlBody => matches!(kind, PartKind::Html) && listed,
            PartRole::CopiedBody => in_text && in_html,
            PartRole::MediaBody => {
                matches!(kind, PartKind::Binary | PartKind::InlineBinary) && listed
            }
            PartRole::Attachment => in_attachments && !listed,
            PartRole::Dropped => !matches!(kind, PartKind::Multipart) && !listed && !in_attachments,
        };
        assert!(consistent, "{context}");
        if part
            .content_disposition()
            .is_some_and(|disposition| disposition.ctype().eq_ignore_ascii_case("attachment"))
        {
            assert!(!part.is_inline(), "{context}");
        }
        if matches!(part.kind(), PartKind::InlineBinary) {
            assert!(part.is_inline(), "{context}");
        }
        if matches!(part.kind(), PartKind::Binary) {
            assert!(!part.is_inline(), "{context}");
        }
    }
    message
        .messages()
        .map(|owner| {
            let dropped: Vec<u32> = owner
                .parts()
                .filter(|part| part.role() == PartRole::Dropped)
                .map(|part| part.id())
                .collect();
            assert_eq!(ids(owner.other_parts()), dropped, "{name}");
            dropped.len()
        })
        .sum()
}

#[test]
fn roles_partition_every_fixture() {
    let files = fixtures();
    assert!(files.len() > 100);
    let mut dropped = Vec::new();
    for path in files {
        let name = path.display().to_string();
        let raw = std::fs::read(&path).expect("fixture");
        for (raw, _) in line_endings(&raw) {
            if let Some(message) = MessageParser::new().parse(&raw)
                && check_roles(&message, &name) > 0
            {
                dropped.push(
                    path.file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned()),
                );
            }
        }
    }
    dropped.dedup();
    assert_eq!(
        dropped,
        DROPPED.map(|name| Some(name.to_string())),
        "only these fixtures have a part in no list"
    );
}
