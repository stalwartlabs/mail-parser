#![feature(test)]

extern crate test;

use std::{fs, path::PathBuf};

use test::Bencher;

fn bench_all_samples(b: &mut Bencher, name: &str, fnc: fn(&mut [u8], &str)) {
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
            let input_str = String::from_utf8_lossy(&test_msg);
            let mut input_bytes = test_msg.clone();

            fnc(&mut input_bytes[..], &input_str);
        }
    });

}

#[bench]
fn bench_stalwart(b: &mut Bencher) {
    bench_all_samples(b, "stalwart_mail_parser", |bytes, _str| {
        mail_parser::Message::parse(bytes);
    });
}

/*

// These libraries do not support all RFCs and might be faster
// on the benchmarks as they skip certain encoded parts or fail 
// while trying to parse the messages.
// Also no body conversion between HTML/text is done by these.
// email = "0.0.21"
// email-format = "0.8"
// email-parser = "0.5.0"
// mailparse = "0.13"

#[bench]
fn bench_mailparse(b: &mut Bencher) {
    bench_all_samples(b, "mailparse", |bytes, _str| {
        mailparse::parse_mail(bytes).unwrap();
    });
}

#[bench]
fn bench_email_parser(b: &mut Bencher) {
    bench_all_samples(b, "email_parser", |bytes, _str| {
        email_parser::prelude::parse_message(bytes).unwrap();
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
