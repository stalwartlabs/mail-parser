mod body;
mod headers;
mod values;

pub use headers::{FORMS, check_headers};
pub use values::{addresses, content_type, datetime, header_value, received, text_list};

use crate::{debug, valid, within};
use mail_parser::{
    Encoding, Message, MessageBuffers, MessageParser, MessagePart, MessageRef, PartId, PartKind,
    PartRole, Source, html_to_text, preview_html, preview_text, text_to_html,
};
use std::{borrow::Cow, ptr};

const PREVIEW_LENS: [usize; 5] = [0, 3, 7, 20, 256];
const CHECKED_BOUNDARIES: usize = 16;
const CHECKED_BODIES: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub depth: usize,
    pub parts: usize,
    pub encoded: usize,
}

pub fn run(parser: &MessageParser, raw: &[u8], limits: Option<Limits>) {
    let parsed = parser.parse(raw);
    let headers_only = parser.parse_headers(raw);
    assert_eq!(parsed.is_some(), headers_only.is_some());
    if let Some(message) = &headers_only {
        check_message(message);
        check_headers_only(message, parsed.as_ref());
        limits
            .into_iter()
            .for_each(|limits| check_limits(message, limits));
    }
    let json = parsed.as_ref().map(|message| {
        check_message(message);
        limits
            .into_iter()
            .for_each(|limits| check_limits(message, limits));
        serialize(message)
    });
    if let Some(message) = parsed {
        assert_eq!(
            Some(serialize(&message.clone().into_owned())),
            json,
            "into_owned"
        );
    }
    assert_eq!(
        parser.parse_owned(raw.to_vec()).as_ref().map(serialize),
        json,
        "parse_owned"
    );
    reuse(parser, raw, json.as_deref());
}

fn serialize(message: &Message<'_>) -> String {
    serde_json::to_string(message).expect("a message serializes")
}

fn reuse(parser: &MessageParser, raw: &[u8], json: Option<&str>) {
    let (_, second) = raw.split_at(raw.len() / 2);
    let second_json = parser.parse(second).as_ref().map(serialize);
    let mut buffers = MessageBuffers::new();
    for (input, expected) in [(raw, json), (second, second_json.as_deref()), (raw, json)] {
        let reused = parser.parse_with(input, &mut buffers);
        assert_eq!(
            reused.as_ref().map(serialize).as_deref(),
            expected,
            "parse_with"
        );
        if let Some(message) = reused {
            buffers = message.into_buffers();
        }
    }
}

fn check_headers_only(message: &Message<'_>, full: Option<&Message<'_>>) {
    assert_eq!(message.messages().len(), 1);
    assert_eq!(message.parts().len(), 1);
    let root = message.root();
    assert_eq!(root.text_body().len(), 0);
    assert_eq!(root.html_body().len(), 0);
    assert_eq!(root.attachments().len(), 0);
    assert!(!root.has_attachments());
    let (part, raw) = (root.root_part(), message.raw());
    assert_eq!(part.offset_end() as usize, raw.len());
    if let Some(full) = full {
        let full_part = full.root().root_part();
        assert_eq!(part.offset_header(), full_part.offset_header());
        assert_eq!(part.offset_body(), full_part.offset_body());
        assert_eq!(
            serde_json::to_string(&part.headers()).ok(),
            serde_json::to_string(&full_part.headers()).ok(),
            "parse_headers and parse disagree on the root header block"
        );
    }
}

pub fn check_message(message: &Message<'_>) {
    for (part, id) in message.parts().zip(0..) {
        assert_eq!(part.id(), id);
        check_part(part);
    }
    let missing = PartId::try_from(message.parts().len()).ok();
    assert!(missing.and_then(|id| message.part(id)).is_none());
    let listed: usize = message.parts().map(|part| part.children().len()).sum();
    let with_parent = message
        .parts()
        .filter(|part| part.parent().is_some())
        .count();
    assert_eq!(
        listed, with_parent,
        "children lists and parent links differ"
    );
    for (nested, id) in message.messages().zip(0..) {
        assert_eq!(nested.id(), id);
        check_ref(message, nested);
    }
    for boundary in message
        .parts()
        .filter_map(|part| part.boundary())
        .take(CHECKED_BOUNDARIES)
    {
        let found = message.part_by_boundary(boundary);
        assert_eq!(found.and_then(|part| part.boundary()), Some(boundary));
    }
    let root = message.root();
    assert_eq!(root.id(), 0);
    assert_eq!(root.source(), Source::Raw);
    assert_eq!(message.subject(), root.subject());
    assert_eq!(message.from(), root.from());
    assert_eq!(message.root_part().id(), root.root_part().id());
    assert_eq!(message.has_attachments(), root.has_attachments());
    assert_eq!(message.body_preview(20), root.body_preview(20));
    assert_eq!(message.body_text(0), root.body_text(0));
    assert_eq!(message.body_html(0), root.body_html(0));
    assert_eq!(message.attachment_name(), root.attachment_name());
    assert_eq!(message.content_type(), root.content_type());
    assert_eq!(message.text_body().len(), root.text_body().len());
    assert_eq!(message.html_body().len(), root.html_body().len());
    assert_eq!(message.attachments().len(), root.attachments().len());
    assert_eq!(message.other_parts().count(), root.other_parts().count());
    assert_eq!(message.headers().len(), root.headers().len());
    headers::getters!(message);
    debug(message);
}

fn check_ref(message: &Message<'_>, nested: MessageRef<'_>) {
    let source = nested.source_bytes();
    match nested.source() {
        Source::Raw => assert!(ptr::eq(source, message.raw())),
        Source::Decoded(id) => {
            let holder = message.part(id).expect("the source part exists");
            assert_ne!(holder.encoding(), Encoding::None);
            assert!(holder.is_message());
            assert_eq!(
                holder.decoded().as_ref(),
                source,
                "source is not the decoded body"
            );
        }
    }
    assert!(within(nested.raw(), source));
    let root = nested.root_part();
    assert_eq!(root.message().id(), nested.id());
    assert!(root.parent().is_none());
    match nested.container() {
        None => assert_eq!(nested.id(), 0),
        Some(container) => {
            assert!(container.is_message());
            assert!(container.id() < root.id());
            assert_eq!(
                container.nested().map(|inner| inner.id()),
                Some(nested.id())
            );
            assert!(
                matches!(container.kind(), PartKind::Message(inner) if inner.id() == nested.id())
            );
        }
    }
    assert_eq!(nested.parts().next().map(|part| part.id()), Some(root.id()));
    let mut previous = None;
    for part in nested.parts() {
        assert_eq!(part.message().id(), nested.id());
        assert!(previous < Some(part.id()));
        previous = Some(part.id());
    }
    check_list(nested, nested.text_body(), MessagePart::in_text_body);
    check_list(nested, nested.html_body(), MessagePart::in_html_body);
    check_list(nested, nested.attachments(), MessagePart::is_attachment);
    assert_eq!(
        nested.has_attachments(),
        nested.attachments().any(|part| {
            !part
                .content_disposition()
                .is_some_and(|disposition| disposition.is_inline())
        })
    );
    assert!(
        nested
            .other_parts()
            .all(|part| part.role() == PartRole::Dropped)
    );
    check_bodies(nested);
    for max_len in PREVIEW_LENS {
        check_preview(nested, max_len);
    }
    assert_eq!(nested.headers().len(), root.headers().len());
    let _ = nested.is_content_type("text", "plain");
    nested.attachment_name().into_iter().for_each(valid);
    [nested.content_type(), nested.content_disposition()]
        .into_iter()
        .flatten()
        .for_each(content_type);
    headers::getters!(nested);
    debug(&nested);
}

fn check_list<'m>(
    owner: MessageRef<'m>,
    list: impl ExactSizeIterator<Item = MessagePart<'m>>,
    member: fn(&MessagePart<'m>) -> bool,
) {
    let len = list.len();
    let mut count = 0;
    for part in list {
        assert_eq!(part.message().id(), owner.id());
        assert!(member(&part));
        count += 1;
    }
    assert_eq!(count, len);
    assert_eq!(owner.parts().filter(member).count(), len);
}

fn as_text(part: MessagePart<'_>) -> Option<Cow<'_, str>> {
    match part.kind() {
        PartKind::Text => part.text(),
        PartKind::Html => part.text().map(|html| Cow::Owned(html_to_text(&html))),
        _ => None,
    }
}

fn as_html(part: MessagePart<'_>) -> Option<Cow<'_, str>> {
    match part.kind() {
        PartKind::Html => part.text(),
        PartKind::Text => part.text().map(|text| Cow::Owned(text_to_html(&text))),
        _ => None,
    }
}

fn check_bodies(owner: MessageRef<'_>) {
    for (index, part) in owner.text_body().enumerate().take(CHECKED_BODIES) {
        assert_eq!(owner.body_text(index), as_text(part));
    }
    for (index, part) in owner.html_body().enumerate().take(CHECKED_BODIES) {
        assert_eq!(owner.body_html(index), as_html(part));
    }
    let texts = owner.text_body().filter_map(as_text);
    assert!(
        owner
            .text_bodies()
            .take(CHECKED_BODIES)
            .eq(texts.take(CHECKED_BODIES))
    );
    let htmls = owner.html_body().filter_map(as_html);
    assert!(
        owner
            .html_bodies()
            .take(CHECKED_BODIES)
            .eq(htmls.take(CHECKED_BODIES))
    );
}

fn check_preview(owner: MessageRef<'_>, max_len: usize) {
    let preview = owner.body_preview(max_len);
    let expected = match (owner.text_body().next(), owner.html_body().next()) {
        (Some(part), _) => match part.kind() {
            PartKind::Text => part
                .text()
                .map(|text| preview_text(&text, max_len).into_owned()),
            PartKind::Html => part.text().map(|html| preview_html(&html, max_len)),
            _ => None,
        },
        (None, Some(part)) => match part.kind() {
            PartKind::Html => part.text().map(|html| preview_html(&html, max_len)),
            PartKind::Text => part
                .text()
                .map(|text| preview_html(&text_to_html(&text), max_len)),
            _ => None,
        },
        (None, None) => None,
    };
    assert_eq!(preview.as_deref(), expected.as_deref(), "body_preview");
    if let Some(preview) = preview {
        valid(&preview);
        assert!(preview.len() <= max_len);
        assert!(!preview.contains('\r'));
    }
}

fn check_part(part: MessagePart<'_>) {
    let source = part.message().source_bytes();
    let (header, body, end) = (
        part.offset_header() as usize,
        part.offset_body() as usize,
        part.offset_end() as usize,
    );
    assert!(
        header <= body && body <= end && end <= source.len(),
        "part {} offsets {header}..{body}..{end} outside its source of {} bytes",
        part.id(),
        source.len()
    );
    assert_eq!(source.get(header..end), Some(part.raw()));
    assert_eq!(source.get(header..body), Some(part.raw_headers()));
    assert_eq!(source.get(body..end), Some(part.raw_body()));
    check_family(part, header, end);
    check_role(part);
    let flags = part.flags();
    assert_eq!(part.has_problems(), !flags.is_empty());
    assert_eq!(flags.names().count(), flags.bits().count_ones() as usize);
    debug(&flags);
    if let Some(boundary) = part.boundary() {
        valid(boundary);
        assert!(part.is_multipart());
        let declared = part.content_type().and_then(|ct| ct.attribute("boundary"));
        assert_eq!(declared, Some(boundary));
    }
    assert_eq!(
        part.delimiter(),
        part.parent().and_then(|parent| parent.boundary())
    );
    check_headers(part.headers(), source, header, body);
    [
        part.attachment_name(),
        part.content_id(),
        part.content_description(),
        part.content_location(),
        part.content_transfer_encoding(),
    ]
    .into_iter()
    .flatten()
    .for_each(valid);
    [part.content_type(), part.content_disposition()]
        .into_iter()
        .flatten()
        .for_each(content_type);
    part.content_language().into_iter().for_each(text_list);
    if let Some(ct) = part.content_type()
        && let Some(subtype) = ct.subtype()
    {
        assert!(part.is_content_type(ct.ctype(), subtype));
    }
    let _ = (part.is_inline(), part.encoding());
    body::check(part);
    debug(&part);
    debug(&part.kind());
}

fn check_family(part: MessagePart<'_>, header: usize, end: usize) {
    if let Some(parent) = part.parent() {
        assert!(parent.is_multipart());
        assert!(parent.id() < part.id());
        assert_eq!(parent.message().id(), part.message().id());
        assert!(
            parent.offset_body() as usize <= header && end <= parent.offset_end() as usize,
            "part {} {header}..{end} outside its parent {} body {}..{}",
            part.id(),
            parent.id(),
            parent.offset_body(),
            parent.offset_end()
        );
    }
    let children = part.children();
    let len = children.len();
    let mut previous = part.id();
    let mut count = 0;
    for child in children {
        assert!(child.id() > previous);
        assert_eq!(child.parent().map(|parent| parent.id()), Some(part.id()));
        previous = child.id();
        count += 1;
    }
    assert_eq!(count, len);
    assert_eq!(part.children().rev().count(), len);
    if !part.is_multipart() {
        assert_eq!(len, 0);
    }
    let kind = part.kind();
    assert_eq!(
        part.is_text(),
        matches!(kind, PartKind::Text | PartKind::Html)
    );
    assert_eq!(part.is_multipart(), matches!(kind, PartKind::Multipart));
    assert_eq!(part.is_message(), matches!(kind, PartKind::Message(_)));
    assert_eq!(part.nested().is_some(), part.is_message());
    if let PartKind::Message(nested) = kind {
        assert_eq!(part.nested().map(|inner| inner.id()), Some(nested.id()));
        assert_eq!(
            nested.container().map(|container| container.id()),
            Some(part.id())
        );
        let root = nested.root_part();
        if ptr::eq(nested.source_bytes(), part.message().source_bytes()) {
            assert!(
                root.offset_header() >= part.offset_body()
                    && root.offset_end() <= part.offset_end(),
                "nested root {} {}..{} outside its container {} body {}..{}",
                root.id(),
                root.offset_header(),
                root.offset_end(),
                part.id(),
                part.offset_body(),
                part.offset_end()
            );
        } else {
            assert_eq!(nested.source(), Source::Decoded(part.id()));
        }
    }
}

fn check_role(part: MessagePart<'_>) {
    let (text, html, attachment) = (
        part.in_text_body(),
        part.in_html_body(),
        part.is_attachment(),
    );
    match part.role() {
        PartRole::Container => {
            assert!(part.is_multipart());
            assert!(!text && !html && !attachment);
        }
        PartRole::CopiedBody => assert!(text && html),
        PartRole::TextBody => {
            assert!(matches!(part.kind(), PartKind::Text));
            assert!(text || html);
        }
        PartRole::HtmlBody => {
            assert!(matches!(part.kind(), PartKind::Html));
            assert!(text || html);
        }
        PartRole::MediaBody => {
            assert!(!part.is_text() && !part.is_multipart());
            assert!(text || html);
        }
        PartRole::Attachment => assert!(attachment && !text && !html),
        PartRole::Dropped => assert!(!text && !html && !attachment),
    }
    debug(&part.role());
}

fn part_depth(part: MessagePart<'_>) -> usize {
    let mut depth = 0;
    let mut current = part;
    loop {
        depth += 1;
        current = match current.parent() {
            Some(parent) => parent,
            None => match current.message().container() {
                Some(container) => container,
                None => return depth,
            },
        };
    }
}

fn encoded_depth(nested: MessageRef<'_>) -> usize {
    let mut depth = 0;
    let mut current = nested.container();
    while let Some(container) = current {
        if container.encoding() != Encoding::None {
            depth += 1;
        }
        current = container.message().container();
    }
    depth
}

pub fn check_limits(message: &Message<'_>, limits: Limits) {
    assert!(message.parts().len() <= limits.parts, "part limit exceeded");
    for part in message.parts() {
        assert!(part_depth(part) <= limits.depth, "depth limit exceeded");
    }
    for nested in message.messages() {
        assert!(
            encoded_depth(nested) <= limits.encoded,
            "encoded nesting limit exceeded"
        );
    }
}
