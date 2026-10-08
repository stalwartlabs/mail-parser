/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{Message, MessageParser, MessagePart, PartKind, preview_html, preview_text};
use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

const LENGTHS: [usize; 16] = [
    0, 1, 2, 3, 7, 10, 31, 50, 100, 255, 256, 257, 1000, 4096, 10_000, 1_000_000,
];

fn eml_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            eml_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "eml") {
            found.push(path);
        }
    }
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

fn first_chars(text: &str, count: usize) -> &str {
    text.char_indices()
        .nth(count)
        .map_or(text, |(end, _)| text.get(..end).unwrap_or(text))
}

fn check_message(message: &Message<'_>, name: &str) -> usize {
    let mut checked = 0;
    for part in message.parts() {
        let Some(text) = part.text() else {
            assert!(part.text_prefix(10).is_none(), "{name}");
            continue;
        };
        for count in LENGTHS {
            let prefix = part.text_prefix(count).expect("a text part");
            assert_eq!(
                prefix,
                first_chars(&text, count),
                "{name} part {} count {count}",
                part.id()
            );
        }
        checked += 1;
    }
    for message in message.messages() {
        for max_len in LENGTHS {
            let expected = if message.text_body().len() > 0 {
                let part = message.text_body().next().expect("text body");
                match part.kind() {
                    PartKind::Text => part
                        .text()
                        .map(|text| preview_text(&text, max_len).into_owned()),
                    PartKind::Html => part.text().map(|html| preview_html(&html, max_len)),
                    _ => None,
                }
            } else if message.html_body().len() > 0 {
                let part = message.html_body().next().expect("html body");
                match part.kind() {
                    PartKind::Html => part.text().map(|html| preview_html(&html, max_len)),
                    PartKind::Text => part
                        .text()
                        .map(|text| preview_html(&mail_parser::text_to_html(&text), max_len)),
                    _ => None,
                }
            } else {
                None
            };
            assert_eq!(
                message.body_preview(max_len).as_deref(),
                expected.as_deref(),
                "{name} preview {max_len}"
            );
        }
    }
    checked
}

#[test]
fn text_prefix_matches_text_on_every_fixture() {
    let mut files = Vec::new();
    eml_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/eml"),
        &mut files,
    );
    files.sort();
    assert!(files.len() > 100, "{}", files.len());
    let mut checked = 0;
    for path in &files {
        let raw = fs::read(path).expect("fixture");
        for raw in line_endings(&raw) {
            let message = MessageParser::new().parse(&raw).expect("parses");
            checked += check_message(&message, &path.display().to_string());
        }
    }
    assert!(checked > 200, "{checked}");
}

fn wrap(encoded: &str, width: usize) -> String {
    encoded
        .as_bytes()
        .chunks(width)
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut word = [0u8; 3];
        word.iter_mut()
            .zip(chunk)
            .for_each(|(slot, &byte)| *slot = byte);
        let bits = u32::from_be_bytes([0, word[0], word[1], word[2]]);
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((bits >> (18 - 6 * index)) & 0x3f) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn quoted_printable(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut line = 0;
    for &byte in bytes {
        let piece = match byte {
            b'\n' => {
                out.push_str("\r\n");
                line = 0;
                continue;
            }
            b'=' | 0x80..=0xff | 0..=0x1f => format!("={byte:02X}"),
            _ => char::from(byte).to_string(),
        };
        if line + piece.len() > 72 {
            out.push_str("=\r\n");
            line = 0;
        }
        line += piece.len();
        out.push_str(&piece);
    }
    out
}

fn encoded_messages() -> Vec<String> {
    let text = "Caf\u{e9} na\u{ef}ve \u{65e5}\u{672c}\u{8a9e} \u{1f600} line\nsecond line with  trailing spaces  \n"
        .repeat(120);
    let mut messages = Vec::new();
    for (charset, bytes) in [
        ("utf-8", text.as_bytes().to_vec()),
        (
            "utf-16",
            [0xff, 0xfe]
                .into_iter()
                .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
                .collect(),
        ),
        (
            "utf-16be",
            text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        ),
        (
            "iso-8859-1",
            text.chars()
                .filter_map(|ch| u8::try_from(u32::from(ch)).ok())
                .collect(),
        ),
    ] {
        for (encoding, body) in [
            ("base64", wrap(&base64(&bytes), 76)),
            ("quoted-printable", quoted_printable(&bytes)),
            ("8bit", String::from_utf8_lossy(&bytes).into_owned()),
        ] {
            if encoding == "8bit" && charset != "utf-8" {
                continue;
            }
            for subtype in ["plain", "html"] {
                messages.push(format!(
                    "Subject: prefix\r\nContent-Type: text/{subtype}; charset={charset}\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{body}"
                ));
            }
        }
    }
    #[cfg(feature = "full_encoding")]
    for label in [
        "shift_jis",
        "euc-jp",
        "iso-2022-jp",
        "gb18030",
        "big5",
        "euc-kr",
    ] {
        let encoding = encoding_rs::Encoding::for_label(label.as_bytes()).expect("label");
        let cjk = "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8} text \u{c548}\u{b155} \u{4e2d}\u{6587}\n"
            .repeat(200);
        let (bytes, _, _) = encoding.encode(&cjk);
        for (encoding, body) in [
            ("base64", wrap(&base64(&bytes), 76)),
            ("quoted-printable", quoted_printable(&bytes)),
        ] {
            messages.push(format!(
                "Subject: prefix\r\nContent-Type: text/plain; charset={label}\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{body}"
            ));
        }
    }
    messages.push(
        "Content-Type: text/plain; charset=utf-7\r\n\r\nHi Mom -+Jjo--! +ZeVnLIqe- tail +AKM"
            .repeat(40),
    );
    messages.push(
        "Content-Type: text/plain\r\nContent-Transfer-Encoding: base64\r\n\r\nSGVs bG8g?garbage=V29y bGQ="
            .to_string(),
    );
    messages.push(
        "Content-Type: text/plain\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nsoft=\r\nbreak =C3=\r\n=A9 end  \r\n=41=\r\n"
            .to_string(),
    );
    messages
}

#[test]
fn text_prefix_on_encoded_bodies() {
    for message in &encoded_messages() {
        let parsed = MessageParser::new()
            .parse(message.as_bytes())
            .expect("parses");
        let name = message.lines().nth(1).unwrap_or_default();
        assert!(check_message(&parsed, name) > 0, "{name}");
    }
}

const EXHAUSTIVE_LIMITS: usize = 64;
const MIN_LIMIT_STRIDE: usize = 61;
const LIMIT_SAMPLES: usize = 64;
const WINDOW_SLACK: usize = 256;
const MAX_UTF8_LEN: usize = 4;
const WHOLE_TEXT_RAW_PER_BYTE: usize = 4;
const DENSE_WHOLE_TEXT_RAW_PER_BYTE: usize = 10;

fn byte_limits(text_len: usize, raw_len: usize) -> impl Iterator<Item = usize> {
    let whole = raw_len.saturating_sub(WINDOW_SLACK) / 2;
    let grown = (raw_len / 4).saturating_sub(WINDOW_SLACK) / 2;
    let guard = raw_len.div_ceil(WHOLE_TEXT_RAW_PER_BYTE);
    let dense_guard = raw_len.div_ceil(DENSE_WHOLE_TEXT_RAW_PER_BYTE);
    (0..=EXHAUSTIVE_LIMITS)
        .chain(LENGTHS)
        .chain((0..=text_len + 1).step_by((text_len / LIMIT_SAMPLES).max(MIN_LIMIT_STRIDE)))
        .chain(text_len.saturating_sub(MAX_UTF8_LEN)..=text_len + MAX_UTF8_LEN)
        .chain(whole.saturating_sub(2)..=whole + 2)
        .chain(grown.saturating_sub(2)..=grown + 2)
        .chain(guard.saturating_sub(1)..=guard + 1)
        .chain(dense_guard.saturating_sub(1)..=dense_guard + 1)
        .chain([usize::MAX])
}

fn floor_prefix(text: &str, max_bytes: usize) -> &str {
    text.get(..text.floor_char_boundary(max_bytes))
        .expect("a char boundary")
}

fn check_prefix_bytes(part: MessagePart<'_>, text: &str, max_bytes: usize, name: &str) {
    let prefix = part.text_prefix_bytes(max_bytes).expect("a text part");
    assert_eq!(
        prefix,
        floor_prefix(text, max_bytes),
        "{name} part {} max_bytes {max_bytes}",
        part.id()
    );
    assert!(
        !part.raw_body().as_ptr_range().contains(&text.as_ptr())
            || matches!(prefix, Cow::Borrowed(_)),
        "{name} part {} max_bytes {max_bytes}: borrowed text, owned prefix",
        part.id()
    );
    if let Some(probe) = max_bytes.checked_add(MAX_UTF8_LEN) {
        let probed = part.text_prefix_bytes(probe).expect("a text part");
        assert_eq!(
            probed.len() > max_bytes,
            text.len() > max_bytes,
            "{name} part {} probe {probe}",
            part.id()
        );
    }
}

fn check_message_bytes(message: &Message<'_>, name: &str) -> usize {
    let mut checked = 0;
    for part in message.parts() {
        let Some(text) = part.text() else {
            assert!(part.text_prefix_bytes(10).is_none(), "{name}");
            continue;
        };
        for max_bytes in byte_limits(text.len(), part.raw_body().len()) {
            check_prefix_bytes(part, &text, max_bytes, name);
        }
        checked += 1;
    }
    checked
}

#[test]
fn text_prefix_bytes_matches_text_on_every_fixture() {
    let mut files = Vec::new();
    eml_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/eml"),
        &mut files,
    );
    files.sort();
    assert!(files.len() > 100, "{}", files.len());
    let mut checked = 0;
    for path in &files {
        let raw = fs::read(path).expect("fixture");
        for raw in line_endings(&raw) {
            let message = MessageParser::new().parse(&raw).expect("parses");
            checked += check_message_bytes(&message, &path.display().to_string());
        }
    }
    assert!(checked > 200, "{checked}");
}

#[test]
fn text_prefix_bytes_on_encoded_bodies() {
    for message in &encoded_messages() {
        let parsed = MessageParser::new()
            .parse(message.as_bytes())
            .expect("parses");
        let name = message.lines().nth(1).unwrap_or_default();
        assert!(check_message_bytes(&parsed, name) > 0, "{name}");
    }
}

fn text_message(encoding: &str, body: &str) -> String {
    format!(
        "Subject: cut\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{body}"
    )
}

fn prefix_window(max_bytes: usize) -> usize {
    max_bytes * 2 + WINDOW_SLACK
}

fn assert_cuts(message: &str, cuts: &[(usize, &str)]) -> usize {
    let parsed = MessageParser::new()
        .parse(message.as_bytes())
        .expect("parses");
    let part = parsed.root_part();
    let text = part.text().expect("a text part");
    for &(max_bytes, expected) in cuts {
        assert_eq!(
            part.text_prefix_bytes(max_bytes).as_deref(),
            Some(expected),
            "max_bytes {max_bytes}"
        );
        check_prefix_bytes(part, &text, max_bytes, "cut");
    }
    part.raw_body().len()
}

#[test]
fn text_prefix_bytes_cut_inside_a_character() {
    let short = "ab\u{65e5}c\u{1f600}d";
    let cuts = [
        (0, ""),
        (1, "a"),
        (2, "ab"),
        (3, "ab"),
        (4, "ab"),
        (5, "ab\u{65e5}"),
        (6, "ab\u{65e5}c"),
        (9, "ab\u{65e5}c"),
        (10, "ab\u{65e5}c\u{1f600}"),
        (11, short),
        (12, short),
        (usize::MAX, short),
    ];
    assert_cuts(&text_message("8bit", short), &cuts);
    assert_cuts(&text_message("base64", "YWLml6Vj8J+YgGQ="), &cuts);
    assert_cuts(
        &text_message("quoted-printable", "ab=E6=97=A5c=F0=9F=98=80d"),
        &cuts,
    );

    let long = "\u{65e5}\u{672c}\u{8a9e}\u{1f600}".repeat(400);
    let body = format!("{long}\r\n");
    let raw_len = assert_cuts(
        &text_message("8bit", &body),
        &[
            (1000, &long[..997]),
            (1001, &long[..1001]),
            (1002, &long[..1001]),
            (1003, &long[..1001]),
            (1004, &long[..1004]),
        ],
    );
    assert!(raw_len > prefix_window(1004));
}

#[test]
fn text_prefix_bytes_cut_inside_a_quoted_printable_escape() {
    let text = "caf\u{e9} cr\u{e8}me br\u{fb}l\u{e9}e \u{e9}t\u{e9}\r\n".repeat(60);
    let mut body = String::new();
    let mut column = 0;
    for byte in text.bytes() {
        if matches!(byte, b'\r' | b'\n') {
            body.push(char::from(byte));
            column = 0;
            continue;
        }
        let piece = if byte.is_ascii() {
            char::from(byte).to_string()
        } else {
            format!("={byte:02X}")
        };
        if column + piece.len() > 75 {
            body.push_str("=\r\n");
            column = 0;
        }
        column += piece.len();
        body.push_str(&piece);
    }
    let message = text_message("quoted-printable", &body);
    let parsed = MessageParser::new()
        .parse(message.as_bytes())
        .expect("parses");
    let part = parsed.root_part();
    let raw = part.raw_body();
    let decoded = part.text().expect("a text part");
    assert_eq!(decoded, text);
    let mut inside_escape = 0;
    for max_bytes in 0..=(raw.len() - WINDOW_SLACK) / 2 {
        let window = raw.get(..prefix_window(max_bytes)).expect("a raw prefix");
        let tail = window
            .get(window.len().saturating_sub(2)..)
            .unwrap_or_default();
        if matches!(tail, [_, b'='] | [b'=', _]) {
            inside_escape += 1;
        }
        check_prefix_bytes(part, &decoded, max_bytes, "qp");
    }
    assert!(inside_escape > 100, "{inside_escape}");
    assert_cuts(
        &message,
        &[
            (3, "caf"),
            (4, "caf"),
            (5, "caf\u{e9}"),
            (9, "caf\u{e9} cr"),
            (10, "caf\u{e9} cr\u{e8}"),
        ],
    );
}

#[test]
fn text_prefix_bytes_cut_inside_a_base64_quad() {
    let text = "Gr\u{fc}\u{df}e aus K\u{f6}ln \u{65e5}\u{672c} \u{1f600}\r\n".repeat(80);
    let encoded = base64(text.as_bytes());
    let body = wrap(&encoded, 76);
    let message = text_message("base64", &body);
    let parsed = MessageParser::new()
        .parse(message.as_bytes())
        .expect("parses");
    let part = parsed.root_part();
    let raw = part.raw_body();
    let decoded = part.text().expect("a text part");
    assert_eq!(decoded, text);
    let mut inside_quad = 0;
    for max_bytes in 0..=(raw.len() - WINDOW_SLACK) / 2 {
        let window = raw.get(..prefix_window(max_bytes)).expect("a raw prefix");
        let symbols = window
            .iter()
            .filter(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
            .count();
        if symbols % 4 != 0 {
            inside_quad += 1;
        }
        check_prefix_bytes(part, &decoded, max_bytes, "base64");
    }
    assert!(inside_quad > 100, "{inside_quad}");
}

#[test]
fn text_prefix_bytes_cut_at_exactly_max_bytes() {
    let text = "exactly\u{e9}";
    let cuts = [(7, "exactly"), (8, "exactly"), (9, text), (10, text)];
    assert_cuts(&text_message("8bit", text), &cuts);
    assert_cuts(&text_message("quoted-printable", "exactly=C3=A9"), &cuts);

    let long = "0123456789".repeat(300);
    let raw_len = assert_cuts(
        &text_message("8bit", &long),
        &[
            (1000, &long[..1000]),
            (1, "0"),
            (3000, &long),
            (2999, &long[..2999]),
        ],
    );
    assert!(raw_len > prefix_window(1000));
}

#[test]
fn text_prefix_bytes_cut_with_crlf_line_endings() {
    let text = "first line\r\nsecond\r\n\r\nthird line\r\n";
    let cuts = [
        (10, "first line"),
        (11, "first line\r"),
        (12, "first line\r\n"),
        (19, "first line\r\nsecond\r"),
        (21, "first line\r\nsecond\r\n\r"),
        (22, "first line\r\nsecond\r\n\r\n"),
        (36, text),
    ];
    assert_cuts(&text_message("8bit", text), &cuts);
    assert_cuts(&text_message("quoted-printable", text), &cuts);
    assert_cuts(&text_message("base64", &base64(text.as_bytes())), &cuts);

    let long = "line of text\r\n".repeat(400);
    let raw_len = assert_cuts(
        &text_message("quoted-printable", &long),
        &[
            (1000, &long[..1000]),
            (1013, &long[..1013]),
            (1014, &long[..1014]),
        ],
    );
    assert!(raw_len > prefix_window(1014));
}

const BASE64_JUNK: [u8; 7] = [b'?', b'*', b'-', b'.', 0, 0xc3, b':'];
const JUNK_PERIOD: usize = 37;
const PADDED_SEGMENTS: usize = 150;
const MIN_SHORT_WINDOWS: usize = 1000;

fn base64_message(body: &[u8]) -> Vec<u8> {
    let mut message = text_message("base64", "").into_bytes();
    message.extend_from_slice(body);
    message
}

fn check_short_windows(message: &[u8], expected: &str, name: &str) -> usize {
    let parsed = MessageParser::new().parse(message).expect("parses");
    let part = parsed.root_part();
    let decoded = part.text().expect("a text part");
    assert_eq!(decoded, expected, "{name}");
    let raw_len = part.raw_body().len();
    let mut checked = 0;
    for max_bytes in (0..).take_while(|&max_bytes| prefix_window(max_bytes) < raw_len) {
        check_prefix_bytes(part, &decoded, max_bytes, name);
        checked += 1;
    }
    check_message_bytes(&parsed, name);
    checked
}

#[test]
fn text_prefix_bytes_on_base64_with_junk() {
    let text = "Gr\u{fc}\u{df}e aus K\u{f6}ln \u{65e5}\u{672c} \u{1f600}\r\n".repeat(120);
    let encoded = wrap(&base64(text.as_bytes()), 76);
    let mut body = Vec::with_capacity(encoded.len() + encoded.len() / JUNK_PERIOD + 1);
    for (chunk, &junk) in encoded
        .as_bytes()
        .chunks(JUNK_PERIOD)
        .zip(BASE64_JUNK.iter().cycle())
    {
        body.extend_from_slice(chunk);
        body.push(junk);
    }
    let checked = check_short_windows(&base64_message(&body), &text, "base64 junk");
    assert!(checked > MIN_SHORT_WINDOWS, "{checked}");
}

#[test]
fn text_prefix_bytes_on_padded_base64_segments() {
    let mut text = String::new();
    let mut body = String::new();
    for index in 0..PADDED_SEGMENTS {
        let segment = format!("segment {index} caf\u{e9} {}\r\n", "x".repeat(index % 7));
        body.push_str(&base64(segment.as_bytes()));
        body.push_str("\r\n");
        text.push_str(&segment);
    }
    let lines = || body.split("\r\n");
    assert!(lines().any(|line| line.ends_with("==")));
    assert!(lines().any(|line| line.ends_with('=') && !line.ends_with("==")));
    let checked = check_short_windows(&base64_message(body.as_bytes()), &text, "base64 segments");
    assert!(checked > MIN_SHORT_WINDOWS, "{checked}");
}

#[test]
fn text_prefix_bytes_on_dense_quoted_printable_at_the_limit() {
    let text = "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{43c}\u{438}\u{440} \u{65e5}\u{672c}\u{8a9e} \u{43e}\u{442}\u{447}\u{451}\u{442}\n"
        .repeat(300);
    let message = text_message("quoted-printable", &quoted_printable(text.as_bytes()));
    let parsed = MessageParser::new()
        .parse(message.as_bytes())
        .expect("parses");
    let part = parsed.root_part();
    let decoded = part.text().expect("a text part");
    let raw_len = part.raw_body().len();
    assert!(raw_len > prefix_window(decoded.len()), "{raw_len}");
    assert!(
        raw_len <= decoded.len() * WHOLE_TEXT_RAW_PER_BYTE,
        "{raw_len}"
    );
    assert_eq!(
        part.text_prefix_bytes(decoded.len()).as_deref(),
        Some(decoded.as_ref())
    );
    let guard = raw_len.div_ceil(WHOLE_TEXT_RAW_PER_BYTE);
    for max_bytes in [
        decoded.len() - 1,
        decoded.len(),
        decoded.len() + 1,
        guard - 1,
        guard,
        guard + 1,
    ] {
        check_prefix_bytes(part, &decoded, max_bytes, "dense qp");
    }
    assert!(check_message_bytes(&parsed, "dense qp") > 0);
}

#[test]
fn text_prefix_bytes_on_quoted_printable_dense_then_sparse() {
    let dense = "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{43c}\u{438}\u{440}\n".repeat(150);
    let sparse = "plain ascii line of a long quoted-printable body\n".repeat(400);
    let message = text_message(
        "quoted-printable",
        &quoted_printable(format!("{dense}{sparse}").as_bytes()),
    );
    let parsed = MessageParser::new()
        .parse(message.as_bytes())
        .expect("parses");
    let part = parsed.root_part();
    let raw = part.raw_body();
    let sample = raw.get(..4096).unwrap_or(raw);
    assert!(sample.iter().filter(|&&byte| byte == b'=').count() * 4 > sample.len());
    assert!(check_message_bytes(&parsed, "dense then sparse qp") > 0);
}

#[test]
fn text_prefix_at_every_character_of_mixed_widths() {
    let text = "a\u{e9}\u{65e5}\u{1f600}bc\u{3b1}\u{20ac} \u{10348}x\n".repeat(90);
    let message = text_message("8bit", &text);
    let parsed = MessageParser::new()
        .parse(message.as_bytes())
        .expect("parses");
    let part = parsed.root_part();
    let chars = text.chars().count();
    for count in 0..=chars + 1 {
        assert_eq!(
            part.text_prefix(count).as_deref(),
            Some(first_chars(&text, count)),
            "{count}"
        );
    }
}
