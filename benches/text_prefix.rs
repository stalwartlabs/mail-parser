/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mail_parser::{Message, MessageParser, MessagePart};
use std::{
    borrow::Cow,
    fs,
    hint::black_box,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

const SIZES: [(&str, usize); 6] = [
    ("4k", 4 << 10),
    ("64k", 64 << 10),
    ("80k", 80 << 10),
    ("128k", 128 << 10),
    ("256k", 256 << 10),
    ("4m", 4 << 20),
];
const PREFIX_CHARS: usize = 65_536;
const PREVIEW_BYTES: usize = 256;
const ATTACHMENT_LIMIT: usize = 65_537;
const BODY_LIMIT: usize = 262_145;
const ATTACHMENT_PROBE: usize = 65_536 + char::MAX_LEN_UTF8;
const BODY_PROBE: usize = 262_144 + char::MAX_LEN_UTF8;
const TEXT_LINE: usize = 72;
const QP_LINE: usize = 76;
const BASE64_LINE_BYTES: usize = 57;

const ASCII_WORDS: &[&str] = &[
    "the",
    "quarterly",
    "report",
    "attached",
    "please",
    "review",
    "meeting",
    "tomorrow",
    "budget",
    "forecast",
    "schedule",
    "project",
    "deadline",
    "customer",
    "invoice",
    "shipping",
    "account",
    "security",
    "regards",
    "release",
];
const UTF8_WORDS: &[&str] = &[
    "the",
    "quarterly",
    "report",
    "attached",
    "please",
    "review",
    "meeting",
    "budget",
    "café",
    "naïve",
    "déjà",
    "résumé",
    "über",
    "Grüße",
    "año",
    "日本語",
    "テキスト",
    "привет",
    "\u{1f600}",
    "regards",
];
const LATIN1_WORDS: &[&str] = &[
    "the",
    "quarterly",
    "report",
    "attached",
    "please",
    "review",
    "meeting",
    "budget",
    "café",
    "naïve",
    "déjà",
    "résumé",
    "über",
    "Grüße",
    "año",
    "façade",
    "smørrebrød",
    "regards",
];
const DENSE_WORDS: &[&str] = &[
    "привет",
    "письмо",
    "сообщение",
    "Москва",
    "日本語",
    "テキスト",
    "中文",
    "отчёт",
];

struct Rng(u64);

impl Rng {
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let index = usize::try_from(self.0 % items.len() as u64).unwrap_or_default();
        items.get(index).copied().unwrap_or_default()
    }
}

fn wrapped_text(words: &[&str], size: usize) -> String {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut text = String::with_capacity(size + TEXT_LINE);
    let mut column = 0;
    while text.len() < size {
        let word = rng.pick(words);
        if column > 0 && column + word.len() + 1 > TEXT_LINE {
            text.push_str("\r\n");
            column = 0;
        } else if column > 0 {
            text.push(' ');
            column += 1;
        }
        text.push_str(word);
        column += word.len();
    }
    text.truncate(text.floor_char_boundary(size));
    text
}

fn base64(bytes: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(bytes.len() / 3 * 4 + bytes.len() / 28 + 8);
    for line in bytes.chunks(BASE64_LINE_BYTES) {
        for chunk in line.chunks(3) {
            let word = chunk.iter().enumerate().fold(0u32, |acc, (index, &byte)| {
                acc | u32::from(byte) << (16 - 8 * index)
            });
            for index in 0..4 {
                out.push(if index <= chunk.len() {
                    ALPHABET[((word >> (18 - 6 * index)) & 0x3f) as usize]
                } else {
                    b'='
                });
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out
}

fn quoted_printable(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 3);
    for (index, line) in text.split("\r\n").enumerate() {
        if index > 0 {
            out.extend_from_slice(b"\r\n");
        }
        let mut column = 0;
        for &byte in line.as_bytes() {
            let escaped = byte == b'=' || !byte.is_ascii();
            let width = if escaped { 3 } else { 1 };
            if column + width > QP_LINE - 1 {
                out.extend_from_slice(b"=\r\n");
                column = 0;
            }
            if escaped {
                write!(out, "={byte:02X}").expect("write to a Vec");
            } else {
                out.push(byte);
            }
            column += width;
        }
    }
    out
}

fn utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|ch| u8::try_from(u32::from(ch)).expect("a Latin-1 character"))
        .collect()
}

fn message(charset: &str, encoding: &str, body: &[u8]) -> Vec<u8> {
    let mut raw = format!(
        "From: sender@example.com\r\nTo: rcpt@example.com\r\nSubject: text prefix\r\nContent-Type: text/plain; charset={charset}\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n"
    )
    .into_bytes();
    raw.extend_from_slice(body);
    raw
}

#[derive(Clone, Copy)]
enum Body {
    Ascii,
    Utf8,
    Base64,
    Base64Utf16,
    QuotedPrintable,
    QuotedPrintableDense,
    Latin1,
}

impl Body {
    const ALL: [Body; 7] = [
        Body::Ascii,
        Body::Utf8,
        Body::Base64,
        Body::Base64Utf16,
        Body::QuotedPrintable,
        Body::QuotedPrintableDense,
        Body::Latin1,
    ];

    fn name(self) -> &'static str {
        match self {
            Body::Ascii => "7bit-ascii",
            Body::Utf8 => "8bit-utf8",
            Body::Base64 => "base64-utf8",
            Body::Base64Utf16 => "base64-utf16",
            Body::QuotedPrintable => "qp-utf8",
            Body::QuotedPrintableDense => "qp-dense-utf8",
            Body::Latin1 => "8bit-latin1",
        }
    }

    fn message(self, size: usize) -> Vec<u8> {
        match self {
            Body::Ascii => message(
                "us-ascii",
                "7bit",
                wrapped_text(ASCII_WORDS, size).as_bytes(),
            ),
            Body::Utf8 => message("utf-8", "8bit", wrapped_text(UTF8_WORDS, size).as_bytes()),
            Body::Base64 => message(
                "utf-8",
                "base64",
                &base64(wrapped_text(UTF8_WORDS, size).as_bytes()),
            ),
            Body::Base64Utf16 => message(
                "utf-16le",
                "base64",
                &base64(&utf16le(&wrapped_text(ASCII_WORDS, size))),
            ),
            Body::QuotedPrintable => message(
                "utf-8",
                "quoted-printable",
                &quoted_printable(&wrapped_text(UTF8_WORDS, size)),
            ),
            Body::QuotedPrintableDense => message(
                "utf-8",
                "quoted-printable",
                &quoted_printable(&wrapped_text(DENSE_WORDS, size)),
            ),
            Body::Latin1 => message(
                "iso-8859-1",
                "8bit",
                &latin1(&wrapped_text(LATIN1_WORDS, size)),
            ),
        }
    }
}

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

fn fixtures() -> Vec<Vec<u8>> {
    let mut paths = Vec::new();
    eml_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/eml"),
        &mut paths,
    );
    paths.sort();
    paths
        .iter()
        .filter_map(|path| fs::read(path).ok())
        .collect()
}

fn text_len(text: Option<Cow<'_, str>>) -> usize {
    text.map_or(0, |text| text.len())
}

fn text(part: MessagePart<'_>) -> usize {
    text_len(part.text())
}

fn text_prefix(part: MessagePart<'_>) -> usize {
    text_len(part.text_prefix(PREFIX_CHARS))
}

fn text_prefix_bytes_attachment(part: MessagePart<'_>) -> usize {
    text_len(part.text_prefix_bytes(ATTACHMENT_LIMIT))
}

fn text_prefix_bytes_body(part: MessagePart<'_>) -> usize {
    text_len(part.text_prefix_bytes(BODY_LIMIT))
}

fn text_prefix_bytes_attachment_probe(part: MessagePart<'_>) -> usize {
    text_len(part.text_prefix_bytes(ATTACHMENT_PROBE))
}

fn text_prefix_bytes_body_probe(part: MessagePart<'_>) -> usize {
    text_len(part.text_prefix_bytes(BODY_PROBE))
}

type PartCase = fn(MessagePart<'_>) -> usize;

const PART_CASES: &[(&str, PartCase)] = &[
    ("text", text),
    ("text_prefix_65536", text_prefix),
    ("text_prefix_bytes_65537", text_prefix_bytes_attachment),
    ("text_prefix_bytes_262145", text_prefix_bytes_body),
    (
        "text_prefix_bytes_65540",
        text_prefix_bytes_attachment_probe,
    ),
    ("text_prefix_bytes_262148", text_prefix_bytes_body_probe),
];

fn body_preview(message: &Message<'_>) -> usize {
    text_len(message.body_preview(PREVIEW_BYTES))
}

fn text_parts(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_prefix");
    for body in Body::ALL {
        for (size_name, size) in SIZES {
            let raw = body.message(size);
            let message = MessageParser::new().parse(&raw).expect("parses");
            let part = message.root_part();
            let input = format!("{}-{size_name}", body.name());
            eprintln!(
                "{input}: raw body {} bytes, text {} bytes",
                part.raw_body().len(),
                text(part)
            );
            for &(name, case) in PART_CASES {
                group.bench_function(BenchmarkId::new(name, &input), |b| {
                    b.iter(|| case(black_box(part)))
                });
            }
            group.bench_function(BenchmarkId::new("body_preview_256", &input), |b| {
                b.iter(|| body_preview(black_box(&message)))
            });
        }
    }
    let raws = fixtures();
    let messages: Vec<Message<'_>> = raws
        .iter()
        .filter_map(|raw| MessageParser::new().parse(raw))
        .collect();
    eprintln!(
        "eml-fixtures: {} messages, {} text parts",
        messages.len(),
        messages
            .iter()
            .flat_map(|message| message.parts())
            .filter(|part| part.is_text())
            .count()
    );
    for &(name, case) in PART_CASES {
        group.bench_function(BenchmarkId::new(name, "eml-fixtures"), |b| {
            b.iter(|| {
                black_box(&messages)
                    .iter()
                    .flat_map(|message| message.parts())
                    .filter(|part| part.is_text())
                    .map(case)
                    .sum::<usize>()
            })
        });
    }
    group.bench_function(BenchmarkId::new("body_preview_256", "eml-fixtures"), |b| {
        b.iter(|| black_box(&messages).iter().map(body_preview).sum::<usize>())
    });
    group.finish();
}

fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(50)
        .noise_threshold(0.02)
}

criterion_group! {
    name = benches;
    config = config();
    targets = text_parts
}
criterion_main!(benches);
