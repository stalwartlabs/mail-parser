/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#[path = "support/corpus.rs"]
mod corpus;

use corpus::{LineEnding, Sample};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mail_parser::{HeaderValue, Message, MessageParser, scan::Kernel};
use memchr::{memchr_iter, memmem};
use std::{fmt::Write, hint::black_box, time::Duration};

fn body_of(sample: &Sample) -> &[u8] {
    memmem::find(&sample.bytes, b"\n\n")
        .or_else(|| memmem::find(&sample.bytes, b"\r\n\r\n"))
        .and_then(|pos| sample.bytes.get(pos..))
        .unwrap_or(&sample.bytes)
}

fn count(hay: &[u8], find: impl Fn(&[u8], usize) -> Option<usize>) -> usize {
    let mut total = 0;
    let mut from = 0;
    while let Some(hit) = find(hay, from) {
        total += 1;
        from = hit + 1;
    }
    total
}

fn dash_by_memchr_dash(hay: &[u8]) -> usize {
    memchr_iter(b'-', hay)
        .filter(|&pos| {
            pos > 0 && hay.get(pos - 1) == Some(&b'\n') && hay.get(pos + 1) == Some(&b'-')
        })
        .count()
}

fn dash_by_memchr_newline(hay: &[u8]) -> usize {
    memchr_iter(b'\n', hay)
        .filter(|&pos| hay.get(pos + 1..pos + 3) == Some(b"--"))
        .count()
}

fn inputs() -> Vec<(String, Vec<u8>)> {
    let synthetic = corpus::synthetic(LineEnding::Crlf);
    let find = |name: &str, sample: &str| {
        synthetic
            .iter()
            .find(|c| c.name.starts_with(name))
            .and_then(|c| c.samples.iter().find(|s| s.name == sample))
            .map(|s| body_of(s).to_vec())
            .unwrap_or_default()
    };
    vec![
        (
            "base64-attachment".to_string(),
            find("attachments", "attachments-2"),
        ),
        (
            "html-newsletter".to_string(),
            find("newsletter", "newsletter-5"),
        ),
        ("text-8bit".to_string(), find("plain-large", "plain-large")),
        ("dash-heavy".to_string(), find("dashes", "dashes")),
    ]
}

fn delimiter_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("delimiter_scan");
    for (name, hay) in inputs() {
        group.throughput(Throughput::Bytes(hay.len() as u64));
        for kernel in Kernel::available() {
            let label = match kernel {
                Kernel::MEMCHR => "memchr-memmem",
                kernel => kernel.name(),
            };
            group.bench_with_input(BenchmarkId::new(label, &name), &hay, |b, h| {
                b.iter(|| count(black_box(h), |hay, from| kernel.dash_line(hay, from)))
            });
        }
        group.bench_with_input(BenchmarkId::new("memchr-dash", &name), &hay, |b, h| {
            b.iter(|| dash_by_memchr_dash(black_box(h)))
        });
        group.bench_with_input(BenchmarkId::new("memchr-newline", &name), &hay, |b, h| {
            b.iter(|| dash_by_memchr_newline(black_box(h)))
        });
    }
    group.finish();
}

fn header_messages(line_ending: LineEnding) -> Vec<Vec<u8>> {
    corpus::synthetic(line_ending)
        .into_iter()
        .find(|c| c.name.starts_with("modern-headers"))
        .map(|c| c.samples.into_iter().map(|s| s.bytes).collect())
        .unwrap_or_default()
}

fn crlf_then_lf(hay: &[u8]) -> Option<usize> {
    memmem::find(hay, b"\r\n\r\n").or_else(|| memmem::find(hay, b"\n\n"))
}

fn fused_blank_line(kernel: Kernel, hay: &[u8]) -> Option<usize> {
    let mut line = 0;
    loop {
        match hay.get(line..)? {
            [b'\n', ..] | [b'\r', b'\n', ..] => return Some(line),
            [] => return None,
            _ => line = kernel.field_end(hay, line)? + 1,
        }
    }
}

const ENRON_BLANK_LINE_SAMPLES: usize = 2;

fn blank_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("blank_line");
    let enron: Vec<Vec<u8>> = corpus::enron(ENRON_BLANK_LINE_SAMPLES)
        .map(|c| c.samples.into_iter().map(|s| s.bytes).collect())
        .unwrap_or_default();
    let inputs = [
        ("modern-lf", header_messages(LineEnding::Lf)),
        ("modern-crlf", header_messages(LineEnding::Crlf)),
        ("enron-crlf", enron),
    ];
    for (name, messages) in inputs {
        if messages.is_empty() {
            continue;
        }
        group.throughput(Throughput::Elements(messages.len() as u64));
        let messages = &messages;
        group.bench_function(BenchmarkId::new("memmem-crlf-then-lf", name), |b| {
            b.iter(|| {
                messages
                    .iter()
                    .filter_map(|m| crlf_then_lf(black_box(m)))
                    .sum::<usize>()
            })
        });
        for kernel in Kernel::available() {
            group.bench_function(
                BenchmarkId::new(format!("fused-{}", kernel.name()), name),
                |b| {
                    b.iter(|| {
                        messages
                            .iter()
                            .filter_map(|m| fused_blank_line(kernel, black_box(m)))
                            .sum::<usize>()
                    })
                },
            );
        }
    }
    group.finish();
}

fn field_split(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_split");
    let blocks: Vec<Vec<u8>> = header_messages(LineEnding::Crlf)
        .into_iter()
        .filter_map(|m| {
            let end = fused_blank_line(Kernel::SCALAR, &m)?;
            m.get(..end).map(<[u8]>::to_vec)
        })
        .collect();
    let bytes: usize = blocks.iter().map(Vec::len).sum();
    group.throughput(Throughput::Bytes(bytes as u64));
    for kernel in Kernel::available() {
        let blocks = &blocks;
        group.bench_function(kernel.name(), |b| {
            b.iter(|| {
                blocks
                    .iter()
                    .map(|block| count(black_box(block), |hay, from| kernel.field_end(hay, from)))
                    .sum::<usize>()
            })
        });
    }
    group.finish();
}

const ADDRESS_LIST_SIZES: [usize; 3] = [8, 64, 512];

fn address_list(mailboxes: usize) -> Vec<u8> {
    let mut out = String::from("From: \"Release Bot (automated)\" <bot@example.com>\r\n");
    for (header, offset) in [("To", 0), ("Cc", 1)] {
        let _ = write!(out, "{header}: ");
        for index in 0..mailboxes {
            if index > 0 {
                out.push_str(",\r\n ");
            }
            let _ = write!(
                out,
                "\"Recipient {index} (team {})\" <user{index}@sub{}.example.org>",
                index % 5,
                (index + offset) % 7
            );
        }
        out.push_str("\r\n");
    }
    out.push_str("Subject: Weekly release\r\nDate: Tue, 3 Sep 2024 10:00:00 +0000\r\n\r\nbody\r\n");
    out.into_bytes()
}

fn addresses_of(message: &Message<'_>) -> usize {
    let mut total = 0;
    for part in message.parts() {
        for header in part.headers().iter() {
            if let HeaderValue::Address(list) = header.value() {
                total += list
                    .mailboxes()
                    .map(|mailbox| {
                        mailbox.name().map_or(0, str::len) + mailbox.address().map_or(0, str::len)
                    })
                    .sum::<usize>();
            }
        }
    }
    total
}

fn header_values(c: &mut Criterion) {
    let parser = MessageParser::new();
    let mut group = c.benchmark_group("header_values");
    let mut inputs: Vec<(String, Vec<Vec<u8>>)> = ADDRESS_LIST_SIZES
        .iter()
        .map(|&size| (format!("address-list-{size}"), vec![address_list(size)]))
        .collect();
    inputs.push((
        "modern-headers".to_string(),
        header_messages(LineEnding::Crlf),
    ));
    for (name, messages) in inputs {
        if messages.is_empty() {
            continue;
        }
        let bytes: usize = messages.iter().map(Vec::len).sum();
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_function(BenchmarkId::new(Kernel::best().name(), name), |b| {
            b.iter(|| {
                messages
                    .iter()
                    .filter_map(|raw| parser.parse_headers(black_box(raw)))
                    .map(|message| addresses_of(&message))
                    .sum::<usize>()
            })
        });
    }
    group.finish();
}

fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(30)
}

criterion_group! {
    name = benches;
    config = config();
    targets = delimiter_scan, blank_line, field_split, header_values
}
criterion_main!(benches);
