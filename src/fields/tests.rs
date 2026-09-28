/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{FieldCtx, HeaderForm, first_field, parse_value};
use crate::{DateTime, HeaderValue, store::MessageData, view::Resolver};
use serde_json::{Value as Json, json};
use std::path::PathBuf;

pub(crate) fn load_tests(name: &str) -> Vec<(String, Json)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join(name);
    let bytes = std::fs::read(&path).expect("fixture file");
    let tests: Vec<Json> = serde_json::from_slice(&bytes).expect("valid JSON");
    tests
        .into_iter()
        .filter_map(|test| {
            Some((
                test.get("header")?.as_str()?.to_string(),
                test.get("expected")?.clone(),
            ))
        })
        .collect()
}

fn content_type_json(value: HeaderValue<'_>) -> Json {
    match value {
        HeaderValue::ContentType(content_type) => {
            let attributes: Vec<Json> = content_type
                .attributes()
                .map(|(name, value)| json!([name, value]))
                .collect();
            json!({
                "c_type": content_type.ctype(),
                "c_subtype": content_type.subtype(),
                "attributes": if attributes.is_empty() { Json::Null } else { Json::Array(attributes) },
            })
        }
        _ => Json::Null,
    }
}

fn normalize(expected: Json) -> Json {
    match expected {
        Json::Object(fields) => {
            let field = |name: &str| fields.get(name).cloned().unwrap_or(Json::Null);
            json!({
                "c_type": field("c_type"),
                "c_subtype": field("c_subtype"),
                "attributes": field("attributes"),
            })
        }
        other => other,
    }
}

#[test]
fn content_type_fixtures() {
    let tests = load_tests("content_type.json");
    assert_eq!(tests.len(), 120);
    for (header, expected) in tests {
        let parsed = HeaderForm::ContentType.parse(header.as_bytes());
        assert_eq!(
            content_type_json(parsed.value()),
            normalize(expected),
            "{header:?}"
        );
    }
}

#[test]
fn raw_values() {
    let inputs = [
        ("Saying Hello\nMessage-Id", Some("Saying Hello")),
        ("Re: Saying Hello\r\n \r\nFrom:", Some("Re: Saying Hello")),
        (
            concat!(
                " from x.y.test\n      by example.net\n      via TCP\n",
                "      with ESMTP\n      id ABC12345\n      ",
                "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600\n"
            ),
            Some(concat!(
                "from x.y.test\n      by example.net\n      via TCP\n",
                "      with ESMTP\n      id ABC12345\n      ",
                "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600"
            )),
        ),
        ("Re: Saying Hello", Some("Re: Saying Hello")),
        ("  \r\n", Some("")),
        ("caf\u{e9}\n", Some("café")),
    ];
    for (input, expected) in inputs {
        let parsed = HeaderForm::Raw.parse(input.as_bytes());
        assert_eq!(parsed.value().as_text(), expected, "{input:?}");
    }
    let parsed = HeaderForm::Raw.parse(b" caf\xe9 \n");
    assert_eq!(parsed.value().as_text(), Some("caf\u{fffd}"));
}

#[test]
fn ignored_values() {
    assert!(HeaderForm::Ignore.parse(b" anything\n").value().is_empty());
}

fn stored(form: HeaderForm, input: &[u8]) -> (bool, Option<String>, Option<DateTime>) {
    let range = first_field(input, 0..input.len());
    let mut data = MessageData::default();
    let value = parse_value(form, &mut FieldCtx::new(input, &mut data), range);
    let value = HeaderValue::new(Resolver::new(input, &data.strings), &data, value);
    (
        value.is_empty(),
        value.as_text().map(str::to_string),
        value.as_datetime(),
    )
}

#[test]
fn standalone_values_match_the_store_path() {
    let mut inputs: Vec<Vec<u8>> = [
        "date.json",
        "unstructured.json",
        "address.json",
        "received.json",
    ]
    .iter()
    .flat_map(|name| load_tests(name))
    .map(|(header, _)| header.into_bytes())
    .collect();
    for sample in [
        &b""[..],
        b" ",
        b"\r\n",
        b" plain value\r\n",
        b"plain\r\n next: x\r\n",
        b" folded\r\n value\r\n",
        b" =?utf-8?q?caf=C3=A9?=\n",
        b" caf\xc3\xa9\n",
        b" caf\xe9\n",
        b" \xff\xfe\r\n",
        b"a\rb\n",
        b"Tue, 1 Jul 2003 10:52:37 +0200\r\n",
        b"Tue, 1 Jul 2003 10:52:37 +0200\nX-y: -0500",
        b" 1 Jul 2003 10:52:37 -0700 (PDT)",
        b"junk\n",
    ] {
        inputs.push(sample.to_vec());
    }
    for input in &inputs {
        for form in [
            HeaderForm::Raw,
            HeaderForm::Text,
            HeaderForm::Date,
            HeaderForm::Ignore,
        ] {
            let parsed = form.parse(input);
            let value = parsed.value();
            assert_eq!(
                (
                    value.is_empty(),
                    value.as_text().map(str::to_string),
                    value.as_datetime()
                ),
                stored(form, input),
                "{form:?} {:?}",
                String::from_utf8_lossy(input)
            );
        }
        let text = String::from_utf8_lossy(input);
        assert_eq!(
            DateTime::parse_rfc822(&text),
            stored(HeaderForm::Date, text.as_bytes()).2,
            "{text:?}"
        );
    }
}
