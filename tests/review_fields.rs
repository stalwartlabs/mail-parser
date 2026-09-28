/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod support;

use mail_parser::{DateTime, HeaderForm, HeaderName, HeaderValue, Message, MessageParser};
use std::{
    borrow::Cow,
    time::{Duration, Instant},
};
use support::regression;

const BUDGET: Duration = Duration::from_secs(3);

fn parse(raw: &[u8]) -> Message<'_> {
    MessageParser::new().parse(raw).expect("message")
}

fn parse_within_budget<'x>(label: &str, raw: &'x [u8]) -> Message<'x> {
    let started = Instant::now();
    let message = parse(raw);
    let elapsed = started.elapsed();
    assert!(
        elapsed < BUDGET,
        "{label}: {elapsed:?} for {} bytes",
        raw.len()
    );
    message
}

#[test]
fn folded_colon_less_lines_are_linear() {
    let mut raw = b"Subject: x\nX".to_vec();
    for _ in 0..200_000 {
        raw.extend_from_slice(b"\n a");
    }
    raw.extend_from_slice(b"\n\nbody\n");
    let message = parse_within_budget("folded colon-less lines", &raw);
    assert_eq!(message.subject(), Some("x"));
    assert_eq!(message.headers().len(), 1);
    assert_eq!(message.root_part().raw_body(), b"body\n");
}

#[test]
fn rfc2231_continuations_are_joined_once_and_capped() {
    let mut raw = b"Content-Type: text/plain; name*0=start".to_vec();
    for index in 1..=45_000 {
        raw.extend_from_slice(format!(";\n name*{index}=abcdefghij").as_bytes());
    }
    raw.extend_from_slice(b"\n\nbody\n");
    let message = parse_within_budget("rfc2231 continuations", &raw);
    let content_type = message.content_type().expect("content type");
    let name = content_type.attribute("name").expect("joined value");
    assert_eq!(name.len(), "start".len() + 999 * "abcdefghij".len());
    assert!(name.starts_with("startabcdefghij"));
    assert_eq!(content_type.attributes().count(), 1);
}

#[test]
fn rfc2231_distinct_sections_and_languages_are_linear() {
    for (label, param) in [
        ("distinct sections", ";\n n{i}*1=v"),
        ("encoded params with a language", ";\n p{i}*0*=utf-8'en'v"),
    ] {
        let mut raw = b"Content-Type: text/plain".to_vec();
        for index in 0..60_000 {
            raw.extend_from_slice(param.replace("{i}", &index.to_string()).as_bytes());
        }
        raw.extend_from_slice(b"\n\nbody\n");
        let message = parse_within_budget(label, &raw);
        let count = message
            .content_type()
            .map_or(0, |content_type| content_type.attributes().count());
        assert!((500..=1_000).contains(&count), "{label}: {count}");
    }

    let mut raw = b"Content-Type: multipart/mixed; boundary=b\n\n".to_vec();
    for _ in 0..40 {
        raw.extend_from_slice(b"--b\nContent-Type: text/plain");
        for index in 0..990 {
            raw.extend_from_slice(format!(";\n n{index}*1=v").as_bytes());
        }
        raw.extend_from_slice(b"\n\nbody\n");
    }
    raw.extend_from_slice(b"--b--\n");
    let message = parse_within_budget("many capped fields", &raw);
    assert_eq!(message.parts().len(), 41);
    for part in message.parts().skip(1) {
        let content_type = part.content_type().expect("content type");
        assert_eq!(content_type.attributes().count(), 990);
        assert_eq!(content_type.attribute("n989"), Some("v"));
    }
}

#[test]
fn empty_text_and_raw_fields_are_empty_strings() {
    for name in [
        "empty-subject",
        "empty-subject-blanks",
        "empty-subject-folded",
    ] {
        let raw = regression(name);
        assert_eq!(parse(&raw).thread_name(), Some(""), "{name}");
    }
    assert_eq!(HeaderForm::Raw.parse(b"").value(), HeaderValue::Text(""));
    assert_eq!(
        HeaderForm::Ignore.parse(b" x\n").value(),
        HeaderValue::Empty
    );
}

fn check_strict_name(raw: &[u8], spelling: &str) {
    let message = parse(raw);
    let from = message
        .from()
        .and_then(|from| from.first())
        .and_then(|mailbox| mailbox.address());
    assert_eq!(from, Some("good@x.com"), "{spelling:?}");
    let odd = message.headers().iter().nth(1).expect("second header");
    assert_eq!(odd.raw_name(), spelling);
    assert_eq!(odd.name(), HeaderName::Other(spelling.into()));
    assert!(message.headers().get(spelling).is_some(), "{spelling:?}");
}

#[test]
fn header_names_are_strict() {
    for (name, spelling) in [
        ("header-name-space", "Fr om"),
        ("header-name-leading-colon", ":From"),
        ("header-name-colons-and-blanks", "::  From"),
        ("header-name-form-feed", "F\x0crom"),
        ("header-name-nul", "Fr\x00om"),
    ] {
        check_strict_name(&regression(name), spelling);
    }
    check_strict_name(b"From: good@x.com\nFrom\r: evil@x.com\n\nbody\n", "From\r");
    let raw = regression("header-name-smuggled-subject");
    assert!(!parse(&raw).headers().has_known());
}

#[test]
fn other_names_spelled_like_known_names_are_normalised() {
    let raw = b"X-Mailer: Ann <ann@example.com>\nSubject: s\nReceived: a\nReceived: b\n\nbody\n";
    let by_other =
        MessageParser::new().header(HeaderName::Other("x-MAILER".into()), HeaderForm::Addresses);
    let message = by_other.parse(raw).expect("message");
    let mailer = message.headers().value(HeaderName::XMailer);
    assert!(
        mailer.and_then(|value| value.as_address()).is_some(),
        "{mailer:?}"
    );
    let ignored = MessageParser::new()
        .header(HeaderName::Other("Content-Type".into()), HeaderForm::Raw)
        .parse(b"Content-Type: text/html\n\nbody")
        .expect("message");
    assert!(ignored.root_part().is_content_type("text", "html"));

    let message = parse(raw);
    let headers = message.headers();
    let other = |name: &'static str| HeaderName::Other(Cow::Borrowed(name));
    assert_eq!(
        headers
            .get(other("X-Mailer"))
            .map(|header| header.raw_name()),
        Some("X-Mailer")
    );
    assert_eq!(
        headers
            .get(other("subject"))
            .map(|header| header.raw_value()),
        Some(&b" s\n"[..])
    );
    assert_eq!(headers.all(other("RECEIVED")).count(), 2);
    assert!(headers.contains(other("received")));
    assert_eq!(other("Subject"), HeaderName::Subject);
    assert_ne!(other("X-Custom"), HeaderName::Subject);
}

#[test]
fn every_field_name_character_can_be_named() {
    let message = parse(b"X.Spam: yes\nX-Tag: t\n\nbody\n");
    assert_eq!(
        message
            .headers()
            .get("X.Spam")
            .map(|header| header.raw_value()),
        Some(&b" yes\n"[..])
    );
    assert_eq!(
        HeaderName::from("X.Spam"),
        HeaderName::Other("X.Spam".into())
    );
    assert_eq!(
        HeaderName::parse("X.Spam"),
        Some(HeaderName::Other("X.Spam".into()))
    );

    let message = MessageParser::new()
        .header("X.Sender", HeaderForm::Addresses)
        .parse(b"X.Sender: Ann <ann@example.com>\n\nbody\n")
        .expect("message");
    let header = message.headers().iter().next().expect("header");
    let address = header
        .value()
        .as_address()
        .and_then(|list| list.first())
        .and_then(|mailbox| mailbox.address());
    assert_eq!(address, Some("ann@example.com"));
    assert!(message.headers().get(header.name()).is_some());
}

#[test]
fn utf16_dangling_byte_is_replaced() {
    let raw = regression("utf16-dangling-byte");
    let message = parse(&raw);
    let part = message.part(1).expect("the first case");
    let (text, problems) = part.text_checked().expect("text part");
    assert_eq!(text, "ab\u{fffd}");
    assert!(!problems.is_empty());
    assert_eq!(part.text_prefix(10).as_deref(), Some("ab\u{fffd}"));
    assert_eq!(part.text_prefix(2).as_deref(), Some("ab"));
}

#[test]
fn to_timezone_saturates() {
    let date = DateTime::from_timestamp(1_637_446_921);
    for tz in [i64::MIN, i64::MIN + 1, i64::MAX, -1, 0, 3_600] {
        let moved = date.to_timezone(tz);
        assert_eq!(moved.tz_before_gmt, tz < 0);
        assert!(moved.tz_minute < 60);
    }
    assert_eq!(date.to_timezone(i64::MIN).tz_hour, u8::MAX);
    assert_eq!(
        date.to_timezone(3_600).to_rfc3339(),
        "2021-11-20T23:22:01+01:00"
    );
}
