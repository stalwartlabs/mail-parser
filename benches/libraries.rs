/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![feature(test)]

extern crate test;

use std::{fs, path::PathBuf};

use test::Bencher;

fn bench_all_samples(b: &mut Bencher, name: &str, fnc: fn(&[u8], &str)) {
    const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----\n".as_bytes();

    println!("Benchmarking {}...\n", name);
    let mut test_data = Vec::new();

    for test_suite in ["rfc", "legacy", "thirdparty", "malformed"] {
        let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_dir.push("tests");
        test_dir.push(test_suite);

        let mut bench_count = 0;

        for file_name in fs::read_dir(&test_dir).unwrap() {
            let file_name = file_name.as_ref().unwrap().path();
            if file_name.extension().map_or(false, |e| e == "txt") {
                let input = fs::read(&file_name).unwrap();
                let mut pos = 0;

                for sep_pos in 0..input.len() {
                    if input[sep_pos..sep_pos + SEPARATOR.len()].eq(SEPARATOR) {
                        pos = sep_pos;
                        break;
                    }
                }

                assert!(
                    pos > 0,
                    "Failed to find separator in test file '{}'.",
                    file_name.display()
                );

                test_data.push(Vec::from(&input[0..pos]));
                bench_count += 1;
            }
        }

        assert!(
            bench_count > 0,
            "Did not find any benchmarks to run in folder {}.",
            test_dir.display()
        );
    }

    b.iter(|| {
        for test_msg in &test_data {
            fnc(test_msg, String::from_utf8_lossy(&test_msg).as_ref());
        }
    });
}

#[bench]
fn bench_stalwart(b: &mut Bencher) {
    bench_all_samples(b, "stalwart_mail_parser", |bytes, _str| {
        mail_parser::Message::parse(bytes);
    });
}

// These libraries do not support all RFCs and might be faster
// on the benchmarks as they do not parse all header fields,
// do not decode encoded parts or fail while trying to parse the messages.
// Also no text body conversion between HTML/plain-text is done by these.
//
// To benchmark against these libraries, add to Cargo.toml dev-dependencies:
//
// email = "0.0.21"
// email-format = "0.8"
// email-parser = { version = "0.5.0", features=["headers", "mime"] }
// mailparse = "0.13"

/*

#[bench]
fn bench_email_parser(b: &mut Bencher) {
    bench_all_samples(b, "email_parser", |bytes, _str| {
        email_parser::prelude::parse_message(bytes).unwrap();
    });
}

#[bench]
fn bench_mailparse(b: &mut Bencher) {
    bench_all_samples(b, "mailparse", |bytes, _str| {
        mailparse::parse_mail(bytes).unwrap();
    });
}

#[bench]
fn benchmark_email(b: &mut Bencher) {
    bench_all_samples(b, "email", |_bytes, str| {
        email::rfc5322::Rfc5322Parser::new(str).consume_message();
    });
}

#[bench]
fn bench_email_format(b: &mut Bencher) {
    use email_format::rfc5322::Parsable;
    bench_all_samples(b, "email_format", |bytes, _str| {
        email_format::Email::parse(bytes).unwrap();
    });
}

*/
