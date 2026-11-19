/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mail_parser::html_to_text;
use std::{hint::black_box, time::Duration};

const SIZES: [(&str, usize); 3] = [("4k", 4 << 10), ("64k", 64 << 10), ("1m", 1 << 20)];

const HEAD: &str = "<html><head><meta charset=\"utf-8\"><title>Weekly</title>\
<style type=\"text/css\">body{margin:0}td{padding:0 8px;font-family:Arial,sans-serif}</style>\
</head><body class=\"main\">";
const TAIL: &str = "</body></html>";

const NEWSLETTER: &[&str] = &[
    "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\"><tr>\
<td style=\"padding:0 8px;color:#333\" class=\"c1\"><a href=\"https://example.com/item?id=42&amp;src=mail\" \
title='Open item'><b>New</b> arrivals this week</a> for you &amp; your team</td></tr></table>\n",
    "<p class=\"intro\" style=\"margin:0 0 12px 0\">Hello <b>Jane</b>, here is what changed since \
your last visit.<br>See <a href=\"https://example.com/a\">the full list</a> online.</p>\n",
    "<img src=\"https://example.com/logo.png\" alt=\"Logo > brand\" width=\"120\" height=\"40\">\n",
    "<div><span style=\"font-size:14px\">Prices start at 10&euro; and ship in 2&ndash;3 days.</span></div>\n",
    "<!--[if mso]><table><tr><td><![endif]--><p>Questions? Reply to this message.</p>\n",
];

const PLAIN: &[&str] = &[
    "<p>The quick brown fox jumps over the lazy dog, and then keeps running down the road.</p>\n",
    "<p>Plain paragraphs with few tags and no attributes at all, mostly text.<br>Second line.</p>\n",
];

fn document(blocks: &[&str], size: usize) -> String {
    let mut html = String::with_capacity(size + HEAD.len() + TAIL.len() + 512);
    html.push_str(HEAD);
    for block in blocks.iter().cycle() {
        if html.len() >= size {
            break;
        }
        html.push_str(block);
    }
    html.push_str(TAIL);
    html
}

fn html(c: &mut Criterion) {
    let mut group = c.benchmark_group("html_to_text");
    for (kind, blocks) in [("newsletter", NEWSLETTER), ("plain", PLAIN)] {
        for (size_name, size) in SIZES {
            let input = document(blocks, size);
            group.throughput(Throughput::Bytes(input.len() as u64));
            group.bench_function(BenchmarkId::new(kind, size_name), |b| {
                b.iter(|| html_to_text(black_box(&input)))
            });
        }
    }
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
    targets = html
}
criterion_main!(benches);
