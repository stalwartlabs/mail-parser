/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(dead_code)]

use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

pub const CORPORA_ENV: &str = "MAIL_PARSER_CORPORA";
const ENRON_MAILDIR: &str = "enron/maildir";
const STALWART_SMTP: &str = "stalwart-smtp";
const SIEVE_MESSAGES: &str = "sieve";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn suffix(self) -> &'static str {
        match self {
            LineEnding::Lf => "lf",
            LineEnding::Crlf => "crlf",
        }
    }

    pub fn apply(self, bytes: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(bytes.len() + bytes.len() / 32);
        match self {
            LineEnding::Lf => result.extend(bytes.iter().filter(|&&byte| byte != b'\r')),
            LineEnding::Crlf => {
                let mut last = 0;
                for &byte in bytes {
                    if byte == b'\n' && last != b'\r' {
                        result.push(b'\r');
                    }
                    result.push(byte);
                    last = byte;
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Corpus {
    pub name: String,
    pub samples: Vec<Sample>,
}

impl Corpus {
    pub fn total_bytes(&self) -> u64 {
        self.samples.iter().map(|s| s.bytes.len() as u64).sum()
    }

    fn with_line_ending(name: &str, line_ending: LineEnding, samples: Vec<Sample>) -> Self {
        Corpus {
            name: format!("{name}-{}", line_ending.suffix()),
            samples: samples
                .into_iter()
                .map(|sample| Sample {
                    bytes: line_ending.apply(&sample.bytes),
                    name: sample.name,
                })
                .collect(),
        }
    }
}

pub fn mail_parser_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn corpora_root() -> PathBuf {
    env::var_os(CORPORA_ENV).map_or_else(
        || mail_parser_root().join("target").join("corpora"),
        PathBuf::from,
    )
}

fn corpus_dir(name: &str) -> Option<PathBuf> {
    let dir = corpora_root().join(name);
    if dir.is_dir() {
        return Some(dir);
    }
    eprintln!(
        "corpus {name} not found in {}: skipped (run scripts/fetch-corpora.sh or set {CORPORA_ENV})",
        dir.display()
    );
    None
}

fn read_dir_sorted(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    paths
}

fn load(paths: impl IntoIterator<Item = PathBuf>) -> Vec<Sample> {
    paths
        .into_iter()
        .filter_map(|path| {
            fs::read(&path).ok().map(|bytes| Sample {
                name: path.display().to_string(),
                bytes,
            })
        })
        .collect()
}

pub fn testsuite_samples() -> Vec<Sample> {
    let resources = mail_parser_root().join("resources");
    let eml = resources.join("eml");
    let suites = ["rfc", "legacy", "thirdparty", "malformed"]
        .iter()
        .flat_map(|suite| read_dir_sorted(&eml.join(suite)))
        .filter(|path| path.extension().is_some_and(|ext| ext == "eml"));
    let sieve = read_dir_sorted(&resources.join(SIEVE_MESSAGES));
    let smtp = corpus_dir(STALWART_SMTP)
        .map(|dir| read_dir_sorted(&dir))
        .unwrap_or_default();
    load(suites.chain(smtp).chain(sieve))
}

pub fn testsuite(line_ending: LineEnding) -> Corpus {
    Corpus::with_line_ending("testsuite", line_ending, testsuite_samples())
}

pub fn enron(per_user: usize) -> Option<Corpus> {
    let users = fs::read_dir(corpus_dir(ENRON_MAILDIR)?).ok()?;
    let mut users: Vec<_> = users.flatten().map(|entry| entry.path()).collect();
    users.sort();
    let mut paths = Vec::new();
    for user in users {
        let mut stack = vec![user];
        let mut taken = 0;
        while let Some(dir) = stack.pop() {
            let mut entries: Vec<_> = fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .collect();
            entries.sort();
            for entry in entries {
                if entry.is_dir() {
                    stack.push(entry);
                } else if taken < per_user {
                    paths.push(entry);
                    taken += 1;
                }
            }
            if taken >= per_user {
                break;
            }
        }
    }
    let samples = load(paths);
    (!samples.is_empty()).then(|| Corpus {
        name: "enron-crlf".to_string(),
        samples,
    })
}

pub fn synthetic(line_ending: LineEnding) -> Vec<Corpus> {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    vec![
        Corpus::with_line_ending("attachments", line_ending, attachments(&mut rng)),
        Corpus::with_line_ending("newsletter", line_ending, newsletters(&mut rng)),
        Corpus::with_line_ending("modern-headers", line_ending, modern_headers(&mut rng)),
        Corpus::with_line_ending("forwarded", line_ending, forwarded(&mut rng)),
        Corpus::with_line_ending("dashes", line_ending, dashes(&mut rng)),
        Corpus::with_line_ending("plain-large", line_ending, plain_large(&mut rng)),
    ]
}

pub fn malformed(line_ending: LineEnding) -> Corpus {
    Corpus::with_line_ending("malformed-synthetic", line_ending, malformed_samples())
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next() % limit as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

const WORDS: &[&str] = &[
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
    "café",
    "naïve",
    "déjà",
    "vu",
    "résumé",
    "日本語",
    "über",
    "schedule",
    "project",
    "deadline",
    "update",
    "customer",
    "invoice",
    "shipping",
    "order",
    "account",
    "security",
    "password",
    "reset",
    "team",
    "lunch",
    "Friday",
    "regards",
    "thanks",
    "server",
    "deploy",
    "release",
    "notes",
    "feature",
    "bug",
    "fix",
    "issue",
];

fn sentence(rng: &mut Rng, words: usize) -> String {
    let mut out = String::new();
    for index in 0..words {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(rng.pick(WORDS));
    }
    out.push('.');
    out
}

fn wrapped_text(rng: &mut Rng, target: usize) -> String {
    let mut out = String::with_capacity(target + 128);
    let mut line = String::new();
    while out.len() < target {
        let word = rng.pick(WORDS);
        if line.len() + word.len() + 1 > 72 {
            out.push_str(&line);
            out.push('\n');
            line.clear();
            if rng.below(12) == 0 {
                out.push('\n');
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push_str(&line);
    out.push('\n');
    out
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = Vec::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let word = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &b)| acc | (b as u32) << (16 - 8 * i));
        let symbols = chunk.len() + 1;
        for index in 0..4 {
            encoded.push(if index < symbols {
                ALPHABET[((word >> (18 - 6 * index)) & 0x3f) as usize]
            } else {
                b'='
            });
        }
    }
    let mut out = String::with_capacity(encoded.len() + encoded.len() / 76 + 2);
    for line in encoded.chunks(76) {
        out.push_str(std::str::from_utf8(line).unwrap_or_default());
        out.push('\n');
    }
    out
}

fn quoted_printable(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 5 / 4);
    for line in text.split('\n') {
        let mut column = 0;
        for &byte in line.as_bytes() {
            let encoded = if byte == b'=' || byte >= 0x80 || byte == b'\t' {
                format!("={byte:02X}")
            } else {
                (byte as char).to_string()
            };
            if column + encoded.len() > 75 {
                out.push_str("=\n");
                column = 0;
            }
            column += encoded.len();
            out.push_str(&encoded);
        }
        out.push('\n');
    }
    out
}

fn html(rng: &mut Rng, target: usize) -> String {
    let mut out = String::with_capacity(target + 512);
    out.push_str(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n<style>\n:root { --brand-color: #0a66c2; --gap: 12px; }\n.button { color: var(--brand-color); margin: var(--gap); }\n</style>\n</head><body>\n",
    );
    while out.len() < target {
        match rng.below(6) {
            0 => {
                let _ = writeln!(out, "<!-- section {} -->", rng.below(1000));
            }
            1 => {
                let _ = writeln!(
                    out,
                    "<table role=\"presentation\" style=\"width:100%;border-top:1px solid #eee\"><tr><td class=\"cell\">{}</td></tr></table>",
                    sentence(rng, 12)
                );
            }
            2 => {
                let _ = writeln!(
                    out,
                    "<a class=\"button\" href=\"https://example.com/track?id={}&amp;u={}\">{}</a>",
                    rng.next(),
                    rng.next(),
                    sentence(rng, 3)
                );
            }
            3 => out.push_str("<hr style=\"border:0;border-top:1px dashed #ccc\">\n-----\n"),
            _ => {
                let _ = writeln!(out, "<p>{}</p>", sentence(rng, 20));
            }
        }
    }
    out.push_str("</body></html>\n");
    out
}

fn envelope(rng: &mut Rng, subject: &str, received: usize, extra: &str) -> String {
    let mut out = String::with_capacity(1024 + received * 256);
    for hop in 0..received {
        let _ = write!(
            out,
            "Received: from mx{hop}.example.net (mx{hop}.example.net [192.0.2.{}])\n\tby relay{hop}.example.org (Postfix) with ESMTPS id {:X}\n\tfor <user@example.org>; Tue, 3 Sep 2024 10:{:02}:{:02} +0000\n",
            rng.below(255),
            rng.next(),
            rng.below(60),
            rng.below(60)
        );
    }
    let _ = write!(
        out,
        "From: \"Sender {}\" <sender{}@example.com>\nTo: Recipient <user@example.org>, \"Another Person\" <another@example.org>\nSubject: {subject}\nDate: Tue, 3 Sep 2024 10:{:02}:{:02} +0000\nMessage-ID: <{:x}@example.com>\nMIME-Version: 1.0\n{extra}",
        rng.below(100),
        rng.below(100),
        rng.below(60),
        rng.below(60),
        rng.next()
    );
    out
}

fn attachments(rng: &mut Rng) -> Vec<Sample> {
    [(400_000usize, 150_000usize), (60_000, 20_000), (2_000_000, 300_000)]
        .into_iter()
        .enumerate()
        .map(|(index, (pdf, png))| {
            let outer = format!("----=_Part_{index}_1634.1725357934");
            let inner = format!("----=_Part_{}_1634.1725357934", index + 10);
            let text = wrapped_text(rng, 4_000);
            let page = html(rng, 30_000);
            let mut out = envelope(
                rng,
                "Quarterly report",
                4,
                &format!("Content-Type: multipart/mixed; boundary=\"{outer}\"\n\nThis is a multi-part message in MIME format.\n\n"),
            );
            let _ = write!(
                out,
                "--{outer}\nContent-Type: multipart/alternative; boundary=\"{inner}\"\n\n--{inner}\nContent-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: quoted-printable\n\n{}--{inner}\nContent-Type: text/html; charset=utf-8\nContent-Transfer-Encoding: quoted-printable\n\n{}--{inner}--\n\n",
                quoted_printable(&text),
                quoted_printable(&page)
            );
            let _ = write!(
                out,
                "--{outer}\nContent-Type: application/pdf; name=\"report.pdf\"\nContent-Disposition: attachment; filename=\"report.pdf\"\nContent-Transfer-Encoding: base64\n\n{}--{outer}\nContent-Type: image/png; name=\"photo.png\"\nContent-Disposition: inline; filename=\"photo.png\"\nContent-ID: <photo{index}@example.com>\nContent-Transfer-Encoding: base64\n\n{}--{outer}--\n",
                base64(&rng.bytes(pdf)),
                base64(&rng.bytes(png))
            );
            Sample {
                name: format!("attachments-{index}"),
                bytes: out.into_bytes(),
            }
        })
        .collect()
}

fn newsletters(rng: &mut Rng) -> Vec<Sample> {
    (0..6)
        .map(|index| {
            let boundary = format!("b1_{:x}", rng.next());
            let related = format!("b2_{:x}", rng.next());
            let mut text = wrapped_text(rng, 8_000 + index * 2_000);
            text.push_str("-----\nUnsubscribe: https://example.com/u\n-- \nThe Newsletter Team\n");
            let page = html(rng, 60_000 + index * 10_000);
            let mut out = envelope(
                rng,
                "Your weekly digest",
                3,
                &format!("Content-Type: multipart/alternative; boundary=\"{boundary}\"\n\n"),
            );
            let _ = write!(
                out,
                "--{boundary}\nContent-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: 8bit\n\n{text}\n--{boundary}\nContent-Type: multipart/related; boundary=\"{related}\"\n\n--{related}\nContent-Type: text/html; charset=utf-8\nContent-Transfer-Encoding: quoted-printable\n\n{}\n--{related}\nContent-Type: image/gif; name=\"logo.gif\"\nContent-ID: <logo@example.com>\nContent-Transfer-Encoding: base64\n\n{}--{related}--\n\n--{boundary}--\n",
                quoted_printable(&page),
                base64(&rng.bytes(12_000))
            );
            Sample {
                name: format!("newsletter-{index}"),
                bytes: out.into_bytes(),
            }
        })
        .collect()
}

fn modern_headers(rng: &mut Rng) -> Vec<Sample> {
    (0..20)
        .map(|index| {
            let boundary = format!("000000000000{:x}", rng.next());
            let signature: String = (0..6)
                .map(|_| format!("\t {}\n", base64(&rng.bytes(48)).trim_end()))
                .collect();
            let extra = format!(
                "ARC-Seal: i=1; a=rsa-sha256; t=1725357934; cv=none; d=example.com; s=arc-20240605;\n{signature}ARC-Message-Signature: i=1; a=rsa-sha256; c=relaxed/relaxed; d=example.com;\n\ts=arc-20240605; h=to:subject:message-id:date:from:mime-version:dkim-signature;\n\tbh={}=;\n{signature}ARC-Authentication-Results: i=1; mx.example.com;\n\tdkim=pass header.i=@example.com header.s=20230601 header.b=abc;\n\tspf=pass (example.com: domain of sender@example.com designates 192.0.2.1 as permitted sender) smtp.mailfrom=sender@example.com;\n\tdmarc=pass (p=NONE sp=QUARANTINE dis=NONE) header.from=example.com\nDKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.com; s=20230601; t=1725357934; x=1725962734;\n\th=to:subject:message-id:date:from:mime-version:from:to:cc:subject:date:message-id:reply-to;\n\tbh={}=;\n{signature}X-Google-Smtp-Source: {}\nX-Received: by 2002:a05:6512:3b0a with SMTP id f10mr{} ; Tue, 03 Sep 2024 03:05:34 -0700 (PDT)\nList-Unsubscribe: <mailto:unsubscribe@example.com?subject=unsub>, <https://example.com/unsub/{:x}>\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\nAuthentication-Results: mx.example.org; dkim=pass; spf=pass; dmarc=pass\nContent-Type: multipart/alternative; boundary=\"{boundary}\"\n\n",
                base64(&rng.bytes(32)).trim_end(),
                base64(&rng.bytes(32)).trim_end(),
                base64(&rng.bytes(40)).trim_end(),
                rng.below(1_000_000),
                rng.next()
            );
            let text = wrapped_text(rng, 1_500);
            let page = html(rng, 6_000);
            let subject = sentence(rng, 6);
            let mut out = envelope(rng, &subject, 6 + index % 4, &extra);
            let _ = write!(
                out,
                "--{boundary}\nContent-Type: text/plain; charset=\"UTF-8\"\nContent-Transfer-Encoding: quoted-printable\n\n{}\n--{boundary}\nContent-Type: text/html; charset=\"UTF-8\"\nContent-Transfer-Encoding: quoted-printable\n\n{}\n--{boundary}--\n",
                quoted_printable(&text),
                quoted_printable(&page)
            );
            Sample {
                name: format!("modern-headers-{index}"),
                bytes: out.into_bytes(),
            }
        })
        .collect()
}

fn nested_message(rng: &mut Rng, depth: usize) -> String {
    let boundary = format!("fwd-{depth}-{:x}", rng.next());
    let mut out = envelope(
        rng,
        &format!("Fwd: level {depth}"),
        2,
        &format!("Content-Type: multipart/mixed; boundary=\"{boundary}\"\n\n"),
    );
    let _ = write!(
        out,
        "--{boundary}\nContent-Type: text/plain; charset=utf-8\n\n{}\n",
        wrapped_text(rng, 1_500)
    );
    if depth == 0 {
        let alternative = format!("alt-{:x}", rng.next());
        let _ = write!(
            out,
            "--{boundary}\nContent-Type: multipart/alternative; boundary=\"{alternative}\"\n\n--{alternative}\nContent-Type: text/plain; charset=utf-8\n\n{}\n--{alternative}\nContent-Type: text/html; charset=utf-8\n\n{}\n--{alternative}--\n\n",
            wrapped_text(rng, 2_000),
            html(rng, 5_000)
        );
    } else {
        let _ = write!(
            out,
            "--{boundary}\nContent-Type: message/rfc822\nContent-Disposition: inline\n\n{}\n",
            nested_message(rng, depth - 1)
        );
    }
    let _ = writeln!(out, "--{boundary}--");
    out
}

fn forwarded(rng: &mut Rng) -> Vec<Sample> {
    (1..=6)
        .map(|depth| Sample {
            name: format!("forwarded-{depth}"),
            bytes: nested_message(rng, depth).into_bytes(),
        })
        .collect()
}

fn dashes(rng: &mut Rng) -> Vec<Sample> {
    let boundary = "==dash-boundary==";
    let mut out = envelope(
        rng,
        "Patch review",
        2,
        &format!("Content-Type: multipart/mixed; boundary=\"{boundary}\"\n\n"),
    );
    for part in 0..3 {
        let mut body = String::with_capacity(200_000);
        while body.len() < 200_000 {
            match rng.below(10) {
                0 => body.push_str("-- SELECT * FROM accounts WHERE id = 42;\n"),
                1 => body.push_str("--- a/src/lib.rs\n+++ b/src/lib.rs\n"),
                2 => {
                    let _ = writeln!(body, "--{boundary}x not a delimiter");
                }
                3 => body.push_str("-----\n"),
                _ => {
                    body.push_str(&sentence(rng, 10));
                    body.push('\n');
                }
            }
        }
        let _ = write!(
            out,
            "--{boundary}\nContent-Type: text/plain; charset=utf-8\nContent-Disposition: attachment; filename=\"part{part}.diff\"\n\n{body}"
        );
    }
    let _ = writeln!(out, "--{boundary}--");
    vec![Sample {
        name: "dashes".to_string(),
        bytes: out.into_bytes(),
    }]
}

fn plain_large(rng: &mut Rng) -> Vec<Sample> {
    let mut body = String::with_capacity(1_000_000);
    while body.len() < 1_000_000 {
        body.push_str(&wrapped_text(rng, 2_000));
        body.push_str("-----Original Message-----\n-- \nSent from my phone\n");
    }
    let mut out = envelope(
        rng,
        "Long thread",
        3,
        "Content-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: 8bit\n\n",
    );
    out.push_str(&body);
    vec![Sample {
        name: "plain-large".to_string(),
        bytes: out.into_bytes(),
    }]
}

fn malformed_samples() -> Vec<Sample> {
    let cases: &[(&str, &str)] = &[
        (
            "inner-missing-close",
            "Subject: a\nContent-Type: multipart/mixed; boundary=\"outer\"\n\n--outer\nContent-Type: multipart/alternative; boundary=\"inner\"\n\n--inner\nContent-Type: text/plain\n\nplain\n--inner\nContent-Type: text/html\n\n<p>html</p>\n--outer\nContent-Type: text/plain\n\nafter the unterminated alternative\n--outer--\n",
        ),
        (
            "prefix-collision",
            "Subject: b\nContent-Type: multipart/mixed; boundary=\"abc\"\n\n--abc\nContent-Type: multipart/alternative; boundary=\"abc-1\"\n\n--abc-1\nContent-Type: text/plain\n\none\n--abc-1\nContent-Type: text/html\n\n<b>two</b>\n--abc-1--\n--abc\nContent-Type: text/plain\n\nthree\n--abc--\n",
        ),
        (
            "part-without-blank-line",
            "Subject: c\nContent-Type: multipart/mixed; boundary=\"x\"\n\n--x\nContent-Type: text/plain\n--x\nContent-Type: text/plain\n\nsecond\n--x--\n",
        ),
        (
            "truncated-base64",
            "Subject: d\nContent-Type: multipart/mixed; boundary=\"t\"\n\n--t\nContent-Type: text/plain\n\nhello\n--t\nContent-Type: application/octet-stream\nContent-Transfer-Encoding: base64\n\nSGVsbG8gd29ybGQ=\nSGVsbG8g",
        ),
        (
            "mid-line-boundary",
            "Subject: e\nContent-Type: multipart/mixed; boundary=\"1\"\n\n--1\nContent-Type: text/plain\n\nfirst\n--1\nContent-Type: text/plain\n\nlast part--1--\n",
        ),
        (
            "empty-rfc822",
            "Subject: f\nContent-Type: multipart/mixed; boundary=\"m\"\n\n--m\nContent-Type: message/rfc822\n\n--m\nContent-Type: text/plain\n\nafter\n--m--\n",
        ),
        (
            "boundary-never-appears",
            "Subject: g\nContent-Type: multipart/mixed; boundary=\"missing\"\n\njust some text\nwithout delimiters\n",
        ),
        ("headers-only", "Subject: h\nFrom: a@example.com"),
        (
            "nested-unterminated-rfc822",
            "Subject: i\nContent-Type: multipart/mixed; boundary=\"o\"\n\n--o\nContent-Type: message/rfc822\n\nSubject: inner\nContent-Type: multipart/mixed; boundary=\"n\"\n\n--n\nContent-Type: text/plain\n\ninner text\n--o\nContent-Type: text/plain\n\nouter again\n--o--\n",
        ),
    ];
    cases
        .iter()
        .map(|(name, text)| Sample {
            name: (*name).to_string(),
            bytes: text.as_bytes().to_vec(),
        })
        .collect()
}
