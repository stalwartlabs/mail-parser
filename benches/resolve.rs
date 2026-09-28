/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#[path = "support/corpus.rs"]
mod corpus;

use corpus::{Corpus, LineEnding};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mail_parser::{HeaderValue, Message, MessageParser};
use std::{hint::black_box, time::Duration};

const ENRON_PER_USER: usize = 2;

fn resolver_name() -> &'static str {
    if cfg!(mail_parser_checked_str) {
        "checked"
    } else {
        "unchecked"
    }
}

fn touch(message: &Message<'_>) -> usize {
    let mut total = 0;
    for part in message.parts() {
        for header in part.headers().iter() {
            total += header.raw_name().len();
            total += match header.value() {
                HeaderValue::Text(text) => text.len(),
                HeaderValue::TextList(list) => list.iter().map(str::len).sum(),
                HeaderValue::Address(list) => list
                    .mailboxes()
                    .map(|mailbox| {
                        mailbox.name().map_or(0, str::len) + mailbox.address().map_or(0, str::len)
                    })
                    .sum(),
                HeaderValue::ContentType(content_type) => {
                    content_type.ctype().len()
                        + content_type
                            .attributes()
                            .map(|(name, value)| name.len() + value.len())
                            .sum::<usize>()
                }
                HeaderValue::Empty | HeaderValue::DateTime(_) | HeaderValue::Received(_) => 0,
            };
        }
    }
    total
}

fn corpora() -> Vec<Corpus> {
    let mut corpora = vec![corpus::testsuite(LineEnding::Crlf)];
    corpora.extend(corpus::enron(ENRON_PER_USER));
    corpora.extend(
        corpus::synthetic(LineEnding::Crlf)
            .into_iter()
            .filter(|corpus| corpus.name.starts_with("modern-headers")),
    );
    corpora
}

fn resolve(c: &mut Criterion) {
    let parser = MessageParser::new();
    let mut group = c.benchmark_group(format!("resolve-{}", resolver_name()));
    for corpus in corpora() {
        let messages: Vec<Message<'_>> = corpus
            .samples
            .iter()
            .filter_map(|sample| parser.parse(&sample.bytes))
            .collect();
        let headers: usize = messages
            .iter()
            .flat_map(|message| message.parts())
            .map(|part| part.headers().len())
            .sum();
        group.throughput(Throughput::Elements(headers as u64));
        group.bench_function(BenchmarkId::new("touch", &corpus.name), |b| {
            b.iter(|| {
                messages
                    .iter()
                    .map(|message| touch(black_box(message)))
                    .sum::<usize>()
            })
        });
        group.bench_function(BenchmarkId::new("parse-and-touch", &corpus.name), |b| {
            b.iter(|| {
                corpus
                    .samples
                    .iter()
                    .filter_map(|sample| parser.parse(black_box(&sample.bytes)))
                    .map(|message| touch(&message))
                    .sum::<usize>()
            })
        });
    }
    group.finish();
}

fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(3))
        .sample_size(30)
}

criterion_group! {
    name = benches;
    config = config();
    targets = resolve
}
criterion_main!(benches);
