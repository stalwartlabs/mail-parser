/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{
    Address, AddressList, AddressRun, Charset, ContentType, DateTime, DecodeProblems, Encoding,
    Greeting, Group, Header, HeaderForm, HeaderIter, HeaderKey, HeaderName, HeaderValue, Headers,
    Host, Mailbox, Message, MessageBuffers, MessageParser, MessagePart, MessageRef, NamedHeaders,
    ParsedValue, PartFlags, PartKind, PartRole, Protocol, Received, Source, TextList, TlsVersion,
};
use std::fmt::Debug;

const MULTIPART: &[u8] = b"From: Ann <ann@example.com>\r\n\
To: team: bob@example.com;\r\n\
Subject: Report\r\n\
Content-Type: multipart/mixed; boundary=\"b\"\r\n\
\r\n\
--b\r\n\
Content-Type: text/plain\r\n\
\r\n\
Hello\r\n\
--b\r\n\
Content-Type: application/pdf; name=report.pdf\r\n\
\r\n\
%PDF\r\n\
--b--\r\n";

fn send_sync<T: Send + Sync>() {}

fn copy<T: Copy + Debug>() {}

fn clone<T: Clone + Debug>() {}

fn eq<T: Eq + Debug>() {}

#[test]
fn public_types_are_send_and_sync() {
    send_sync::<Message<'static>>();
    send_sync::<MessageParser>();
    send_sync::<MessageBuffers>();
    send_sync::<MessageRef<'static>>();
    send_sync::<MessagePart<'static>>();
    send_sync::<Headers<'static>>();
    send_sync::<Header<'static>>();
    send_sync::<NamedHeaders<'static, 'static>>();
    send_sync::<HeaderIter<'static>>();
    send_sync::<HeaderKey<'static>>();
    send_sync::<HeaderValue<'static>>();
    send_sync::<HeaderName<'static>>();
    send_sync::<AddressList<'static>>();
    send_sync::<Address<'static>>();
    send_sync::<Mailbox<'static>>();
    send_sync::<Group<'static>>();
    send_sync::<AddressRun<'static>>();
    send_sync::<TextList<'static>>();
    send_sync::<ContentType<'static>>();
    send_sync::<Received<'static>>();
    send_sync::<Host<'static>>();
    send_sync::<PartKind<'static>>();
    send_sync::<ParsedValue<'static>>();
}

#[test]
fn views_and_small_values_are_copy() {
    copy::<MessageRef<'static>>();
    copy::<MessagePart<'static>>();
    copy::<Headers<'static>>();
    copy::<Header<'static>>();
    copy::<HeaderValue<'static>>();
    copy::<AddressList<'static>>();
    copy::<Address<'static>>();
    copy::<Mailbox<'static>>();
    copy::<Group<'static>>();
    copy::<AddressRun<'static>>();
    copy::<TextList<'static>>();
    copy::<ContentType<'static>>();
    copy::<Received<'static>>();
    copy::<Host<'static>>();
    copy::<PartKind<'static>>();
    copy::<PartRole>();
    copy::<PartFlags>();
    copy::<DecodeProblems>();
    copy::<Source>();
    copy::<Encoding>();
    copy::<HeaderForm>();
    copy::<Charset>();
    copy::<DateTime>();
    copy::<Protocol>();
    copy::<Greeting>();
    copy::<TlsVersion>();
    clone::<Message<'static>>();
    clone::<MessageParser>();
    clone::<MessageBuffers>();
    clone::<ParsedValue<'static>>();
    clone::<NamedHeaders<'static, 'static>>();
    clone::<HeaderName<'static>>();
}

#[test]
fn values_compare_by_content() {
    eq::<HeaderValue<'static>>();
    eq::<AddressList<'static>>();
    eq::<TextList<'static>>();
    eq::<ContentType<'static>>();
    eq::<Received<'static>>();
    eq::<MessageParser>();
    eq::<ParsedValue<'static>>();
    let parser = MessageParser::new();
    let first = parser.parse(MULTIPART).expect("message");
    let copy = MULTIPART.to_vec();
    let second = parser.parse(&copy).expect("message");
    for name in [
        HeaderName::From,
        HeaderName::To,
        HeaderName::Subject,
        HeaderName::ContentType,
    ] {
        let value = first.headers().value(name.clone());
        assert!(value.is_some_and(|value| !value.is_empty()));
        assert_eq!(value, second.headers().value(name));
    }
    assert_ne!(
        first.headers().value(HeaderName::From),
        first.headers().value(HeaderName::To)
    );
    let received = b" from a (b [10.0.0.1]) by c with ESMTPS id 1; Sat, 20 Nov 2021 14:22:01 -0800";
    let parsed = HeaderForm::Received.parse(received);
    let again = HeaderForm::Received.parse(received);
    assert_eq!(parsed, again);
    assert_eq!(parsed.value(), again.value());
    assert_ne!(parsed, HeaderForm::Raw.parse(received));
    assert_eq!(
        parsed
            .value()
            .as_received()
            .and_then(|received| received.with()),
        Some(Protocol::ESMTPS)
    );
}

fn shorten_message<'short, 'long: 'short>(message: Message<'long>) -> Message<'short> {
    message
}

fn shorten_part<'short, 'long: 'short>(part: MessagePart<'long>) -> MessagePart<'short> {
    part
}

fn shorten_value<'short, 'long: 'short>(value: HeaderValue<'long>) -> HeaderValue<'short> {
    value
}

fn shorten_name<'short, 'long: 'short>(name: HeaderName<'long>) -> HeaderName<'short> {
    name
}

#[test]
fn message_is_covariant() {
    let parser = MessageParser::new();
    let owned: Message<'static> = parser.parse_owned(MULTIPART.to_vec()).expect("message");
    let local = b"Subject: local\r\n\r\nbody".to_vec();
    let mut message = parser.parse(&local).expect("message");
    assert_eq!(message.subject(), Some("local"));
    message = owned;
    assert_eq!(message.subject(), Some("Report"));
    let message = shorten_message(message);
    let part = shorten_part(message.root_part());
    assert_eq!(part.children().len(), 2);
    let value = shorten_value(
        part.headers()
            .value(HeaderName::Subject)
            .unwrap_or_default(),
    );
    assert_eq!(value.as_text(), Some("Report"));
    assert!(shorten_name(HeaderName::Other("X-Local".into())).is_other());
}

#[test]
fn owned_messages_move_across_threads() {
    let message = MessageParser::new()
        .parse(MULTIPART)
        .expect("message")
        .into_owned();
    let subject = std::thread::spawn(move || message.subject().map(str::to_string))
        .join()
        .expect("thread");
    assert_eq!(subject.as_deref(), Some("Report"));
}

#[test]
fn headers_only_messages() {
    let parser = MessageParser::new();
    let message = parser.parse_headers(MULTIPART).expect("message");
    let root = message.root_part();
    assert_eq!(message.parts().len(), 1);
    assert!(matches!(root.kind(), PartKind::Multipart));
    assert_eq!(root.children().len(), 0);
    assert_eq!(root.role(), PartRole::Container);
    assert_eq!(root.offset_end() as usize, MULTIPART.len());
    assert_eq!(message.text_body().len(), 0);
    assert_eq!(message.html_body().len(), 0);
    assert_eq!(message.attachments().len(), 0);
    assert_eq!(message.other_parts().count(), 0);
    assert!(!message.has_attachments());
    assert_eq!(message.subject(), Some("Report"));

    let single = b"Subject: plain\r\n\r\nHello\r\n";
    let message = parser.parse_headers(single).expect("message");
    let root = message.root_part();
    assert!(matches!(root.kind(), PartKind::Text));
    assert_eq!(root.role(), PartRole::Dropped);
    assert_eq!(
        message
            .other_parts()
            .map(|part| part.id())
            .collect::<Vec<_>>(),
        [0]
    );
    assert!(!message.has_attachments());
    assert_eq!(root.text().as_deref(), Some("Hello\r\n"));
}

#[test]
fn buffers_stay_usable_after_inputs_that_do_not_parse() {
    let parser = MessageParser::new();
    let mut buffers = MessageBuffers::new();
    let mut subjects = Vec::new();
    for raw in [
        MULTIPART,
        b"".as_slice(),
        b"Subject: second\r\n\r\nbody".as_slice(),
    ] {
        match parser.parse_with(raw, &mut buffers) {
            Some(message) => {
                subjects.push(message.subject().map(str::to_string));
                buffers = message.into_buffers();
            }
            None => subjects.push(None),
        }
    }
    assert_eq!(
        subjects,
        [Some("Report".to_string()), None, Some("second".to_string())]
    );
}

#[test]
fn debug_output_shows_content() {
    let message = MessageParser::new().parse(MULTIPART).expect("message");
    let headers = format!("{:?}", message.headers());
    assert!(headers.contains("name: \"Subject\", value: Text(\"Report\")"));
    assert!(headers.len() < 2_000);
    assert_eq!(
        format!("{:?}", message.from().expect("from")),
        "[Mailbox(Mailbox { name: Some(\"Ann\"), address: Some(\"ann@example.com\") })]"
    );
    assert_eq!(
        format!("{:?}", message.to().expect("to")),
        "[Group(Group { name: Some(\"team\"), members: [Mailbox { name: None, address: Some(\"bob@example.com\") }] })]"
    );
    assert_eq!(
        format!("{:?}", message.content_type().expect("content type")),
        "ContentType { type: \"multipart\", subtype: Some(\"mixed\"), attributes: {\"boundary\": \"b\"} }"
    );
    let parsed = HeaderForm::MessageIds.parse(b" <a@b> <c@d>");
    assert_eq!(
        format!("{parsed:?}"),
        "ParsedValue { value: TextList([\"a@b\", \"c@d\"]) }"
    );
    assert_eq!(
        format!("{:?}", MessageBuffers::new()),
        "MessageBuffers { .. }"
    );
}

#[test]
fn headers_iterate_in_for_loops() {
    let message = MessageParser::new().parse(MULTIPART).expect("message");
    let headers = message.headers();
    let mut names = Vec::new();
    for header in headers {
        names.push(header.raw_name());
    }
    assert_eq!(names, ["From", "To", "Subject", "Content-Type"]);
    let by_ref: Vec<_> = (&headers)
        .into_iter()
        .map(|header| header.raw_name())
        .collect();
    assert_eq!(by_ref, names);
    let iter: HeaderIter<'_> = headers.iter();
    assert_eq!(iter.len(), 4);
    assert_eq!(
        headers.iter().rev().map(|header| header.raw_name()).next(),
        Some("Content-Type")
    );
    assert_eq!(
        format!("{:?}", headers.iter()),
        "HeaderIter { remaining: 4, .. }"
    );
}

#[test]
fn all_received_lists_every_trace_field() {
    let raw = b"Received: from a.example by b.example; Tue, 1 Jul 2003 10:52:37 +0200\r\n\
Received: from c.example by d.example; Tue, 1 Jul 2003 10:52:36 +0200\r\n\
Subject: trace\r\n\r\nbody\r\n";
    let message = MessageParser::new().parse(&raw[..]).expect("message");
    let hosts: Vec<_> = message
        .all_received()
        .map(|received| received.from().map(|host| host.to_string()))
        .collect();
    assert_eq!(
        hosts,
        [Some("a.example".to_string()), Some("c.example".to_string())]
    );
}

#[test]
fn header_keys_find_what_names_find() {
    let message = MessageParser::new().parse(MULTIPART).expect("message");
    let headers = message.headers();
    for name in [
        HeaderName::Subject,
        HeaderName::from("subject"),
        HeaderName::Other("SUBJECT".into()),
        HeaderName::from("x-missing"),
        HeaderName::from("Content-Type"),
    ] {
        let key: HeaderKey<'_> = name.key();
        let copy = key;
        assert_eq!(copy, key);
        assert_eq!(
            headers
                .all_key(key)
                .map(|h| h.raw_value())
                .collect::<Vec<_>>(),
            headers
                .all(name.clone())
                .map(|h| h.raw_value())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            headers.get_key(key).map(|h| h.raw_value()),
            headers.get(name.clone()).map(|h| h.raw_value())
        );
    }
    let content_type = HeaderName::from("content-type");
    let key = content_type.key();
    for part in message.parts() {
        assert_eq!(
            part.headers().get_key(key).is_some(),
            part.headers().get("Content-Type").is_some()
        );
    }
}
