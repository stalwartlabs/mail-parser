/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{MessageParser, PartKind, strip_charset_meta};
use std::{borrow::Cow, path::PathBuf};

const EM_DASH: char = '\u{2014}';

const FIXTURES: [(&str, &str, &str); 5] = [
    (
        "109.eml",
        "content=\"text/html; charset=Windows-1252\"",
        "content=\"text/html; charset=utf-8\"",
    ),
    (
        "109-charset.eml",
        "<meta charset=Windows-1252>",
        "<meta charset=utf-8>",
    ),
    (
        "109-charset-quoted.eml",
        "<meta charset=\"Windows-1252\">",
        "<meta charset=\"utf-8\">",
    ),
    (
        "109-http-equiv.eml",
        "content=\"text/html; charset=Windows-1252\"",
        "content=\"text/html; charset=utf-8\"",
    ),
    (
        "109-http-equiv-quot.eml",
        "charset=&quot;Windows-1252&quot;",
        "charset=&quot;utf-8&quot;",
    ),
];

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/eml/issues")
            .join(name),
    )
    .expect("fixture exists")
}

fn line_endings(raw: &[u8]) -> [Vec<u8>; 2] {
    let lf: Vec<u8> = raw.iter().copied().filter(|&byte| byte != b'\r').collect();
    let mut crlf = Vec::with_capacity(lf.len() * 2);
    for &byte in &lf {
        if byte == b'\n' {
            crlf.push(b'\r');
        }
        crlf.push(byte);
    }
    [lf, crlf]
}

#[test]
fn issue_109_charset_declarations() {
    for (name, declared, rewritten) in FIXTURES {
        for raw in line_endings(&fixture(name)) {
            let message = MessageParser::new()
                .parse(&raw)
                .expect("the message parses");
            let html_part = message
                .html_body()
                .next()
                .expect("an HTML body part exists");
            assert!(matches!(html_part.kind(), PartKind::Html), "{name}");

            let html = html_part.text().expect("the HTML part has text");
            assert!(html.contains(declared), "{name}: {html:?}");
            assert!(html.contains(EM_DASH), "{name}: {html:?}");

            let utf8 = html_part.html_utf8().expect("an HTML part");
            assert!(!utf8.contains(declared), "{name}: {utf8:?}");
            assert!(utf8.contains(rewritten), "{name}: {utf8:?}");
            assert!(
                !utf8.to_ascii_lowercase().contains("windows-1252"),
                "{name}: {utf8:?}"
            );
            assert!(utf8.contains(EM_DASH), "{name}: {utf8:?}");
            assert_eq!(
                utf8.replacen(rewritten, declared, 1),
                html.as_ref(),
                "{name}: only the declaration changes"
            );

            assert_eq!(
                message.body_html(0).as_deref(),
                Some(html.as_ref()),
                "{name}: the default output keeps the declaration"
            );

            let text_part = message.text_body().next().expect("a text body part exists");
            assert!(matches!(text_part.kind(), PartKind::Text), "{name}");
            assert!(
                text_part.text().is_some_and(|text| text.contains(EM_DASH)),
                "{name}"
            );
            assert!(text_part.html_utf8().is_none(), "{name}");
        }
    }
}

#[test]
fn issue_109_unchanged_html_is_borrowed() {
    for html in [
        "<html><head><meta charset=\"utf-8\"></head><body>\u{2014}</body></html>",
        "<html><head></head><body><meta charset=Windows-1252></body></html>",
        "no markup",
    ] {
        assert!(
            matches!(strip_charset_meta(html), Cow::Borrowed(_)),
            "{html}"
        );
    }
}
