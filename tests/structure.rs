/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod support;

use mail_parser::{MessageParser, MessagePart, MessageRef, PartKind, PartRole, Source};
use serde_json::{Map, Value, json};
use std::fs;
use support::{snapshot_inputs, snapshot_messages};

const MESSAGE_FIELDS: [&str; 8] = [
    "id",
    "root",
    "container",
    "source",
    "text_body",
    "html_body",
    "attachments",
    "has_attachments",
];
const PART_FIELDS: [&str; 12] = [
    "id",
    "message",
    "parent",
    "kind",
    "children",
    "nested",
    "flags",
    "roles",
    "role",
    "offset_header",
    "offset_body",
    "offset_end",
];
const HEADER_FIELDS: [&str; 4] = ["name", "offset_field", "offset_start", "offset_end"];

fn ids<'m>(parts: impl Iterator<Item = MessagePart<'m>>) -> Vec<u32> {
    parts.map(|part| part.id()).collect()
}

fn message_json(message: MessageRef<'_>) -> Value {
    json!({
        "id": message.id(),
        "root": message.root_part().id(),
        "container": message.container().map(|part| part.id()),
        "source": match message.source() {
            Source::Raw => json!("raw"),
            Source::Decoded(part) => json!({ "decoded": part }),
        },
        "text_body": ids(message.text_body()),
        "html_body": ids(message.html_body()),
        "attachments": ids(message.attachments()),
        "has_attachments": message.has_attachments(),
    })
}

fn kind_name(kind: PartKind<'_>) -> &'static str {
    match kind {
        PartKind::Text => "text",
        PartKind::Html => "html",
        PartKind::Binary => "binary",
        PartKind::InlineBinary => "inline_binary",
        PartKind::Multipart => "multipart",
        PartKind::Message(_) => "message",
    }
}

fn role_name(role: PartRole) -> &'static str {
    match role {
        PartRole::Container => "container",
        PartRole::TextBody => "text_body",
        PartRole::HtmlBody => "html_body",
        PartRole::CopiedBody => "copied_body",
        PartRole::MediaBody => "media_body",
        PartRole::Attachment => "attachment",
        PartRole::Dropped => "dropped",
    }
}

fn part_json(part: MessagePart<'_>) -> Value {
    let roles = [
        (part.in_text_body(), "text_body"),
        (part.in_html_body(), "html_body"),
        (part.is_attachment(), "attachment"),
        (part.is_inline(), "inline"),
    ];
    let mut fields = json!({
        "id": part.id(),
        "message": part.message().id(),
        "parent": part.parent().map(|parent| parent.id()),
        "kind": kind_name(part.kind()),
        "flags": part.flags().names().collect::<Vec<_>>(),
        "roles": roles
            .iter()
            .filter(|(set, _)| *set)
            .map(|(_, name)| *name)
            .collect::<Vec<_>>(),
        "role": role_name(part.role()),
        "offset_header": part.offset_header(),
        "offset_body": part.offset_body(),
        "offset_end": part.offset_end(),
        "headers": part
            .headers()
            .iter()
            .map(|header| {
                json!({
                    "name": header.raw_name(),
                    "offset_field": header.offset_field(),
                    "offset_start": header.offset_start(),
                    "offset_end": header.offset_end(),
                })
            })
            .collect::<Vec<_>>(),
    });
    let extra = match part.kind() {
        PartKind::Multipart => Some(("children", json!(ids(part.children())))),
        PartKind::Message(nested) => Some(("nested", json!(nested.id()))),
        _ => None,
    };
    if let (Some((name, value)), Some(object)) = (extra, fields.as_object_mut()) {
        object.insert(name.to_string(), value);
    }
    fields
}

fn project(value: &Value, names: &[&str]) -> Value {
    Value::Object(
        names
            .iter()
            .filter_map(|name| Some((name.to_string(), value.get(*name)?.clone())))
            .collect::<Map<_, _>>(),
    )
}

fn expected_part(part: &Value) -> Value {
    let mut fields = project(part, &PART_FIELDS);
    let headers: Vec<Value> = part
        .get("headers")
        .and_then(Value::as_array)
        .map(|headers| {
            headers
                .iter()
                .map(|header| project(header, &HEADER_FIELDS))
                .collect()
        })
        .unwrap_or_default();
    if let Some(object) = fields.as_object_mut() {
        object.insert("headers".to_string(), Value::Array(headers));
    }
    fields
}

#[test]
fn structure_matches_the_snapshots() {
    let parser = MessageParser::new();
    let mut checked = 0;
    for path in snapshot_messages() {
        let raw = fs::read(&path).expect("the message is readable");
        for (input, extension) in snapshot_inputs(&raw) {
            let snapshot: Value = serde_json::from_slice(
                &fs::read(path.with_extension(extension)).expect("the snapshot exists"),
            )
            .expect("the snapshot is JSON");
            check(
                &parser,
                &input,
                &snapshot,
                &format!("{} ({extension})", path.display()),
            );
            checked += 1;
        }
    }
    assert!(checked > 300, "{checked}");
}

fn check(parser: &MessageParser, input: &[u8], snapshot: &Value, label: &str) {
    let Some(message) = parser.parse(input) else {
        assert!(snapshot.is_null(), "{label}");
        return;
    };
    let expected_messages: Vec<Value> = snapshot
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .map(|message| project(message, &MESSAGE_FIELDS))
                .collect()
        })
        .unwrap_or_default();
    let messages: Vec<Value> = message.messages().map(message_json).collect();
    assert_eq!(messages, expected_messages, "{label}");
    let expected_parts: Vec<Value> = snapshot
        .get("parts")
        .and_then(Value::as_array)
        .map(|parts| parts.iter().map(expected_part).collect())
        .unwrap_or_default();
    let parts: Vec<Value> = message.parts().map(part_json).collect();
    assert_eq!(parts, expected_parts, "{label}");
}
