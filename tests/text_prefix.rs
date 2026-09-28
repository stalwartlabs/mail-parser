/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{Message, MessageParser, PartKind, preview_html, preview_text};
use std::{
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

#[test]
fn text_prefix_on_encoded_bodies() {
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
    for message in &messages {
        let parsed = MessageParser::new()
            .parse(message.as_bytes())
            .expect("parses");
        let name = message.lines().nth(1).unwrap_or_default();
        assert!(check_message(&parsed, name) > 0, "{name}");
    }
}
