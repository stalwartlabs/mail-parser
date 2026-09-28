use super::Sample;
use std::{fmt::Write as _, ops::RangeInclusive};

pub(super) const NAMES: [&str; 6] = [
    "attachments",
    "newsletter",
    "modern-headers",
    "forwarded",
    "dashes",
    "plain-large",
];

const SEED: u64 = 0x9e37_79b9_7f4a_7c15;
const LINE_WIDTH: usize = 72;
const QP_WIDTH: usize = 75;
const BASE64_WIDTH: usize = 76;
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const ATTACHMENT_SIZES: [(usize, usize); 3] =
    [(400_000, 150_000), (60_000, 20_000), (2_000_000, 300_000)];
const FORWARD_DEPTHS: RangeInclusive<usize> = 1..=6;
const DASH_BOUNDARY: &str = "==dash-boundary==";
const DASH_PART_SIZE: usize = 200_000;
const PLAIN_LARGE_SIZE: usize = 1_000_000;

const HTML_HEAD: &str = "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n<style>\n:root { --brand-color: #0a66c2; --gap: 12px; }\n.button { color: var(--brand-color); margin: var(--gap); }\n</style>\n</head><body>\n";

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

pub(super) fn generate() -> [Vec<Sample>; 6] {
    let mut rng = Rng(SEED);
    [
        attachments(&mut rng),
        newsletters(&mut rng),
        modern_headers(&mut rng),
        forwarded(&mut rng),
        dashes(&mut rng),
        plain_large(&mut rng),
    ]
}

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next_u64() % limit as u64) as usize
    }

    fn pick(&mut self, items: &[&'static str]) -> &'static str {
        items
            .get(self.below(items.len()))
            .copied()
            .unwrap_or_default()
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }
}

fn push_sentence(out: &mut String, rng: &mut Rng, words: usize) {
    for index in 0..words {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(rng.pick(WORDS));
    }
    out.push('.');
}

fn wrapped_text(rng: &mut Rng, target: usize) -> String {
    let mut out = String::with_capacity(target + 128);
    let mut line = String::with_capacity(LINE_WIDTH + 16);
    while out.len() < target {
        let word = rng.pick(WORDS);
        if line.len() + word.len() + 1 > LINE_WIDTH {
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
    let mut encoded = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk
            .iter()
            .zip([16u32, 8, 0])
            .fold(0u32, |acc, (&byte, shift)| acc | u32::from(byte) << shift);
        let symbols = chunk.len() + 1;
        encoded.extend(
            [18u32, 12, 6, 0]
                .into_iter()
                .enumerate()
                .map(|(index, shift)| {
                    if index < symbols {
                        BASE64_ALPHABET
                            .get(((word >> shift) & 0x3f) as usize)
                            .copied()
                            .unwrap_or(b'=')
                    } else {
                        b'='
                    }
                }),
        );
    }
    let mut out = String::with_capacity(encoded.len() + encoded.len() / BASE64_WIDTH + 1);
    for line in encoded.chunks(BASE64_WIDTH) {
        out.extend(line.iter().copied().map(char::from));
        out.push('\n');
    }
    out
}

fn quoted_printable(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 5 / 4);
    for line in text.split('\n') {
        let mut column = 0;
        for byte in line.bytes() {
            let escaped = byte == b'=' || byte >= 0x80 || byte == b'\t';
            let width = if escaped { 3 } else { 1 };
            if column + width > QP_WIDTH {
                out.push_str("=\n");
                column = 0;
            }
            column += width;
            if escaped {
                let _ = write!(out, "={byte:02X}");
            } else {
                out.push(char::from(byte));
            }
        }
        out.push('\n');
    }
    out
}

fn html(rng: &mut Rng, target: usize) -> String {
    let mut out = String::with_capacity(target + 512);
    out.push_str(HTML_HEAD);
    while out.len() < target {
        match rng.below(6) {
            0 => {
                let section = rng.below(1000);
                let _ = writeln!(out, "<!-- section {section} -->");
            }
            1 => {
                out.push_str("<table role=\"presentation\" style=\"width:100%;border-top:1px solid #eee\"><tr><td class=\"cell\">");
                push_sentence(&mut out, rng, 12);
                out.push_str("</td></tr></table>\n");
            }
            2 => {
                let id = rng.next_u64();
                let user = rng.next_u64();
                let _ = write!(
                    out,
                    "<a class=\"button\" href=\"https://example.com/track?id={id}&amp;u={user}\">"
                );
                push_sentence(&mut out, rng, 3);
                out.push_str("</a>\n");
            }
            3 => out.push_str("<hr style=\"border:0;border-top:1px dashed #ccc\">\n-----\n"),
            _ => {
                out.push_str("<p>");
                push_sentence(&mut out, rng, 20);
                out.push_str("</p>\n");
            }
        }
    }
    out.push_str("</body></html>\n");
    out
}

fn envelope(rng: &mut Rng, subject: &str, received: usize, extra: &str) -> String {
    let mut out = String::with_capacity(1024 + received * 256);
    for hop in 0..received {
        let ip = rng.below(255);
        let id = rng.next_u64();
        let minute = rng.below(60);
        let second = rng.below(60);
        let _ = write!(
            out,
            "Received: from mx{hop}.example.net (mx{hop}.example.net [192.0.2.{ip}])\n\tby relay{hop}.example.org (Postfix) with ESMTPS id {id:X}\n\tfor <user@example.org>; Tue, 3 Sep 2024 10:{minute:02}:{second:02} +0000\n"
        );
    }
    let sender_name = rng.below(100);
    let sender = rng.below(100);
    let minute = rng.below(60);
    let second = rng.below(60);
    let id = rng.next_u64();
    let _ = write!(
        out,
        "From: \"Sender {sender_name}\" <sender{sender}@example.com>\nTo: Recipient <user@example.org>, \"Another Person\" <another@example.org>\nSubject: {subject}\nDate: Tue, 3 Sep 2024 10:{minute:02}:{second:02} +0000\nMessage-ID: <{id:x}@example.com>\nMIME-Version: 1.0\n{extra}"
    );
    out
}

fn attachments(rng: &mut Rng) -> Vec<Sample> {
    ATTACHMENT_SIZES
        .into_iter()
        .enumerate()
        .map(|(index, (pdf_size, png_size))| {
            let outer = format!("----=_Part_{index}_1634.1725357934");
            let inner = format!("----=_Part_{}_1634.1725357934", index + 10);
            let text = quoted_printable(&wrapped_text(rng, 4_000));
            let page = quoted_printable(&html(rng, 30_000));
            let mut out = envelope(
                rng,
                "Quarterly report",
                4,
                &format!("Content-Type: multipart/mixed; boundary=\"{outer}\"\n\nThis is a multi-part message in MIME format.\n\n"),
            );
            let _ = write!(
                out,
                "--{outer}\nContent-Type: multipart/alternative; boundary=\"{inner}\"\n\n--{inner}\nContent-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: quoted-printable\n\n{text}--{inner}\nContent-Type: text/html; charset=utf-8\nContent-Transfer-Encoding: quoted-printable\n\n{page}--{inner}--\n\n"
            );
            let pdf = base64(&rng.bytes(pdf_size));
            let png = base64(&rng.bytes(png_size));
            let _ = write!(
                out,
                "--{outer}\nContent-Type: application/pdf; name=\"report.pdf\"\nContent-Disposition: attachment; filename=\"report.pdf\"\nContent-Transfer-Encoding: base64\n\n{pdf}--{outer}\nContent-Type: image/png; name=\"photo.png\"\nContent-Disposition: inline; filename=\"photo.png\"\nContent-ID: <photo{index}@example.com>\nContent-Transfer-Encoding: base64\n\n{png}--{outer}--\n"
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
            let boundary = format!("b1_{:x}", rng.next_u64());
            let related = format!("b2_{:x}", rng.next_u64());
            let mut text = wrapped_text(rng, 8_000 + index * 2_000);
            text.push_str("-----\nUnsubscribe: https://example.com/u\n-- \nThe Newsletter Team\n");
            let page = quoted_printable(&html(rng, 60_000 + index * 10_000));
            let mut out = envelope(
                rng,
                "Your weekly digest",
                3,
                &format!("Content-Type: multipart/alternative; boundary=\"{boundary}\"\n\n"),
            );
            let logo = base64(&rng.bytes(12_000));
            let _ = write!(
                out,
                "--{boundary}\nContent-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: 8bit\n\n{text}\n--{boundary}\nContent-Type: multipart/related; boundary=\"{related}\"\n\n--{related}\nContent-Type: text/html; charset=utf-8\nContent-Transfer-Encoding: quoted-printable\n\n{page}\n--{related}\nContent-Type: image/gif; name=\"logo.gif\"\nContent-ID: <logo@example.com>\nContent-Transfer-Encoding: base64\n\n{logo}--{related}--\n\n--{boundary}--\n"
            );
            Sample {
                name: format!("newsletter-{index}"),
                bytes: out.into_bytes(),
            }
        })
        .collect()
}

fn signature_block(rng: &mut Rng) -> String {
    let mut out = String::with_capacity(6 * 68);
    for _ in 0..6 {
        let line = base64(&rng.bytes(48));
        let _ = writeln!(out, "\t {}", line.trim_end());
    }
    out
}

fn modern_headers(rng: &mut Rng) -> Vec<Sample> {
    (0..20)
        .map(|index| {
            let boundary = format!("000000000000{:x}", rng.next_u64());
            let signature = signature_block(rng);
            let arc_hash = base64(&rng.bytes(32));
            let dkim_hash = base64(&rng.bytes(32));
            let smtp_source = base64(&rng.bytes(40));
            let smtp_id = rng.below(1_000_000);
            let unsubscribe = rng.next_u64();
            let extra = format!(
                "ARC-Seal: i=1; a=rsa-sha256; t=1725357934; cv=none; d=example.com; s=arc-20240605;\n{signature}ARC-Message-Signature: i=1; a=rsa-sha256; c=relaxed/relaxed; d=example.com;\n\ts=arc-20240605; h=to:subject:message-id:date:from:mime-version:dkim-signature;\n\tbh={}=;\n{signature}ARC-Authentication-Results: i=1; mx.example.com;\n\tdkim=pass header.i=@example.com header.s=20230601 header.b=abc;\n\tspf=pass (example.com: domain of sender@example.com designates 192.0.2.1 as permitted sender) smtp.mailfrom=sender@example.com;\n\tdmarc=pass (p=NONE sp=QUARANTINE dis=NONE) header.from=example.com\nDKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.com; s=20230601; t=1725357934; x=1725962734;\n\th=to:subject:message-id:date:from:mime-version:from:to:cc:subject:date:message-id:reply-to;\n\tbh={}=;\n{signature}X-Google-Smtp-Source: {}\nX-Received: by 2002:a05:6512:3b0a with SMTP id f10mr{smtp_id} ; Tue, 03 Sep 2024 03:05:34 -0700 (PDT)\nList-Unsubscribe: <mailto:unsubscribe@example.com?subject=unsub>, <https://example.com/unsub/{unsubscribe:x}>\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\nAuthentication-Results: mx.example.org; dkim=pass; spf=pass; dmarc=pass\nContent-Type: multipart/alternative; boundary=\"{boundary}\"\n\n",
                arc_hash.trim_end(),
                dkim_hash.trim_end(),
                smtp_source.trim_end(),
            );
            let text = quoted_printable(&wrapped_text(rng, 1_500));
            let page = quoted_printable(&html(rng, 6_000));
            let mut subject = String::with_capacity(96);
            push_sentence(&mut subject, rng, 6);
            let mut out = envelope(rng, &subject, 6 + index % 4, &extra);
            let _ = write!(
                out,
                "--{boundary}\nContent-Type: text/plain; charset=\"UTF-8\"\nContent-Transfer-Encoding: quoted-printable\n\n{text}\n--{boundary}\nContent-Type: text/html; charset=\"UTF-8\"\nContent-Transfer-Encoding: quoted-printable\n\n{page}\n--{boundary}--\n"
            );
            Sample {
                name: format!("modern-headers-{index}"),
                bytes: out.into_bytes(),
            }
        })
        .collect()
}

fn nested_message(rng: &mut Rng, depth: usize) -> String {
    let mut out = String::new();
    let mut boundaries = Vec::with_capacity(depth + 1);
    for level in (0..=depth).rev() {
        let boundary = format!("fwd-{level}-{:x}", rng.next_u64());
        out.push_str(&envelope(
            rng,
            &format!("Fwd: level {level}"),
            2,
            &format!("Content-Type: multipart/mixed; boundary=\"{boundary}\"\n\n"),
        ));
        let text = wrapped_text(rng, 1_500);
        let _ = write!(
            out,
            "--{boundary}\nContent-Type: text/plain; charset=utf-8\n\n{text}\n"
        );
        if level == 0 {
            let alternative = format!("alt-{:x}", rng.next_u64());
            let plain = wrapped_text(rng, 2_000);
            let page = html(rng, 5_000);
            let _ = write!(
                out,
                "--{boundary}\nContent-Type: multipart/alternative; boundary=\"{alternative}\"\n\n--{alternative}\nContent-Type: text/plain; charset=utf-8\n\n{plain}\n--{alternative}\nContent-Type: text/html; charset=utf-8\n\n{page}\n--{alternative}--\n\n"
            );
        } else {
            let _ = write!(
                out,
                "--{boundary}\nContent-Type: message/rfc822\nContent-Disposition: inline\n\n"
            );
        }
        boundaries.push(boundary);
    }
    for (index, boundary) in boundaries.iter().rev().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "--{boundary}--");
    }
    out
}

fn forwarded(rng: &mut Rng) -> Vec<Sample> {
    FORWARD_DEPTHS
        .map(|depth| Sample {
            name: format!("forwarded-{depth}"),
            bytes: nested_message(rng, depth).into_bytes(),
        })
        .collect()
}

fn dashes(rng: &mut Rng) -> Vec<Sample> {
    let mut out = envelope(
        rng,
        "Patch review",
        2,
        &format!("Content-Type: multipart/mixed; boundary=\"{DASH_BOUNDARY}\"\n\n"),
    );
    for part in 0..3 {
        let _ = write!(
            out,
            "--{DASH_BOUNDARY}\nContent-Type: text/plain; charset=utf-8\nContent-Disposition: attachment; filename=\"part{part}.diff\"\n\n"
        );
        let start = out.len();
        while out.len() - start < DASH_PART_SIZE {
            match rng.below(10) {
                0 => out.push_str("-- SELECT * FROM accounts WHERE id = 42;\n"),
                1 => out.push_str("--- a/src/lib.rs\n+++ b/src/lib.rs\n"),
                2 => {
                    let _ = writeln!(out, "--{DASH_BOUNDARY}x not a delimiter");
                }
                3 => out.push_str("-----\n"),
                _ => {
                    push_sentence(&mut out, rng, 10);
                    out.push('\n');
                }
            }
        }
    }
    let _ = writeln!(out, "--{DASH_BOUNDARY}--");
    vec![Sample {
        name: "dashes".to_string(),
        bytes: out.into_bytes(),
    }]
}

fn plain_large(rng: &mut Rng) -> Vec<Sample> {
    let mut body = String::with_capacity(PLAIN_LARGE_SIZE + 4_096);
    while body.len() < PLAIN_LARGE_SIZE {
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
