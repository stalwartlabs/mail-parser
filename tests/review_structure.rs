/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod support;

use mail_parser::{
    Message, MessageBuffers, MessageParser, MessagePart, PartFlags, PartKind, Source,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use support::{line_endings, resource};

const CHAIN_BUDGET: Duration = Duration::from_secs(5);
const NOISE_LINE: &[u8] = b"\n--X";

fn parse(raw: &[u8]) -> Message<'_> {
    MessageParser::new().parse(raw).expect("the input parses")
}

fn flags(part: MessagePart<'_>) -> Vec<&'static str> {
    part.flags().names().collect()
}

fn check_nesting(message: &Message<'_>) {
    for part in message.parts() {
        let label = format!("part {}", part.id());
        assert!(part.offset_header() <= part.offset_body(), "{label}");
        assert!(part.offset_body() <= part.offset_end(), "{label}");
        assert!(
            part.offset_end() as usize <= part.message().source_bytes().len(),
            "{label}"
        );
        let container = part
            .parent()
            .or_else(|| part.message().container())
            .filter(|container| container.message().source() == part.message().source());
        if let Some(container) = container {
            assert!(
                container.offset_body() <= part.offset_header(),
                "{label} starts before its container {}",
                container.id()
            );
            assert!(
                part.offset_end() <= container.offset_end(),
                "{label} ends past its container {}",
                container.id()
            );
        }
    }
}

fn chain(root: &[u8], part: &[u8], parts: usize, noise: usize, tail: &[u8]) -> Vec<u8> {
    let mut raw = root.to_vec();
    for _ in 0..parts {
        raw.extend_from_slice(part);
    }
    for _ in 0..noise {
        raw.extend_from_slice(NOISE_LINE);
    }
    raw.extend_from_slice(tail);
    raw
}

fn fixtures() -> Vec<Vec<u8>> {
    let mut files = Vec::new();
    for suite in [
        "rfc",
        "legacy",
        "thirdparty",
        "malformed",
        "issues",
        "regressions",
    ] {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/eml")
            .join(suite);
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("the suite directory exists")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "eml"))
            .collect();
        paths.sort_unstable();
        files.extend(paths.iter().filter_map(|path| std::fs::read(path).ok()));
    }
    files
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound.max(1) as u64) as usize
    }
}

#[test]
fn security_bug_input_keeps_its_content() {
    let raw = resource("malformed/023.eml");
    for (input, _) in line_endings(&raw) {
        let message = parse(&input);
        check_nesting(&message);
        let root = message.root_part();
        assert!(matches!(root.kind(), PartKind::Multipart));
        assert_eq!(flags(root), ["fallback_delimiter"]);
        let children: Vec<MessagePart<'_>> = root.children().collect();
        assert_eq!(children.len(), 1);
        let child = children.first().copied().expect("one child");
        assert!(matches!(child.kind(), PartKind::Text));
        assert!(child.flags().is_empty());
        let text = child.text().expect("text");
        assert_eq!(text.replace('\r', ""), "5\nContent-Type:message/rfc822\n\n");
        assert_eq!(message.text_body().len(), 1);
        for part in message.parts() {
            let _ = (part.text(), part.decoded(), part.text_checked());
            let _ = (part.raw(), part.raw_headers(), part.raw_body());
        }
        let _ = message.body_preview(64);
        let headers = MessageParser::new()
            .parse_headers(&input)
            .expect("the headers parse");
        assert_eq!(headers.parts().len(), 1);
    }
}

#[test]
fn parts_stay_inside_their_containers() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let inserts: [&[u8]; 6] = [
        b"\n--",
        b"\n\n",
        b"\nContent-Type: message/rfc822\n\n",
        b"\nContent-Type: multipart/mixed; boundary=x\n\n--x\n",
        b"\nContent-Transfer-Encoding: base64\n",
        b"--",
    ];
    for file in fixtures() {
        for (input, _) in line_endings(&file) {
            if let Some(message) = MessageParser::new().parse(&input) {
                check_nesting(&message);
            }
            for _ in 0..12 {
                let mut mutated = input.clone();
                for _ in 0..=rng.below(3) {
                    let at = rng.below(mutated.len() + 1);
                    match rng.below(3) {
                        0 => {
                            let end = (at + rng.below(64)).min(mutated.len());
                            mutated.drain(at..end);
                        }
                        1 => {
                            let insert = inserts.get(rng.below(inserts.len())).copied();
                            let tail = mutated.split_off(at);
                            mutated.extend_from_slice(insert.unwrap_or_default());
                            mutated.extend_from_slice(&tail);
                        }
                        _ => mutated.truncate(at),
                    }
                }
                if let Some(message) = MessageParser::new().parse(&mutated) {
                    check_nesting(&message);
                }
            }
        }
    }
}

#[test]
fn fallback_chains_are_linear() {
    let root = b"Content-Type: multipart/mixed; boundary=B\n\n";
    let encoded_part = b"x--B\nContent-Type: message/rfc822\nContent-Transfer-Encoding: base64\n\nU3ViamVjdDogcw0KDQpib2R5DQo=\n";
    let cases: [(&str, Vec<u8>, usize); 5] = [
        (
            "fallback chain",
            chain(root, b"x--B\n\n", 4_000, 400_000, b"\n"),
            4_001,
        ),
        (
            "message/rfc822 fallback chain",
            chain(
                root,
                b"x--B\nContent-Type: message/rfc822\n\nSubject: s\n\nbody\n",
                4_000,
                400_000,
                b"\n",
            ),
            8_001,
        ),
        (
            "nested fallback chain",
            chain(
                b"Content-Type: multipart/mixed; boundary=A\n\n--A\nContent-Type: multipart/mixed; boundary=B\n\n",
                b"x--B\n\n",
                4_000,
                400_000,
                b"\n--A--\n",
            ),
            4_002,
        ),
        (
            "encoded message/rfc822 fallback chain",
            chain(root, encoded_part, 2_000, 400_000, b"\n"),
            4_001,
        ),
        (
            "chain of multiparts whose boundary never occurs",
            chain(
                root,
                b"x--B\nContent-Type: multipart/mixed; boundary=Z\n\ny\n",
                4_000,
                400_000,
                b"\n",
            ),
            4_001,
        ),
    ];
    for (name, raw, parts) in cases {
        let start = Instant::now();
        let message = parse(&raw);
        let elapsed = start.elapsed();
        assert_eq!(message.parts().len(), parts, "{name}");
        check_nesting(&message);
        assert!(
            elapsed < CHAIN_BUDGET,
            "{name}: {} bytes took {elapsed:?}",
            raw.len()
        );
    }
}

fn digest(count: usize) -> Vec<u8> {
    let mut raw = b"Content-Type: multipart/digest; boundary=d\r\n\r\n".to_vec();
    for index in 0..count {
        raw.extend_from_slice(
            format!("--d\r\n\r\nSubject: {index}\r\n\r\nbody {index}\r\n").as_bytes(),
        );
    }
    raw.extend_from_slice(b"--d--\r\n");
    raw
}

fn limited(raw: &[u8], parts: usize) -> Message<'_> {
    MessageParser::new()
        .max_parts(parts)
        .parse(raw)
        .expect("the input parses")
}

fn flagged(message: &Message<'_>, flag: PartFlags) -> Vec<u32> {
    message
        .parts()
        .filter(|part| part.flags().contains(flag))
        .map(|part| part.id())
        .collect()
}

#[test]
fn part_limit_flags() {
    let raw = digest(5);
    let message = limited(&raw, 2);
    check_nesting(&message);
    assert!(message.root_part().flags().is_empty());
    let cut = message.part(1).expect("cut container");
    assert!(matches!(cut.kind(), PartKind::Text));
    assert_eq!(flags(cut), ["limit_reached"]);
    assert_eq!(cut.offset_end() as usize, raw.len());

    let message = limited(&raw, 3);
    check_nesting(&message);
    assert_eq!(flags(message.root_part()), ["limit_reached"]);
    let nested_root = message.part(2).expect("nested root");
    assert!(nested_root.flags().is_empty());
    assert_eq!(
        nested_root.offset_end(),
        message.part(1).expect("container").offset_end()
    );

    let message = limited(&raw, 4);
    check_nesting(&message);
    assert_eq!(flagged(&message, PartFlags::LIMIT_REACHED), [3]);
    assert!(message.root_part().flags().is_empty());

    let mut mixed = b"Content-Type: multipart/mixed; boundary=d\r\n\r\n".to_vec();
    for index in 0..5 {
        mixed.extend_from_slice(format!("--d\r\n\r\nbody {index}\r\n").as_bytes());
    }
    mixed.extend_from_slice(b"--d--\r\n");
    let message = limited(&mixed, 3);
    check_nesting(&message);
    assert_eq!(flagged(&message, PartFlags::LIMIT_REACHED), [2]);
    let last = message.part(2).expect("last");
    assert_eq!(last.offset_end() as usize, mixed.len());
    assert!(message.root_part().flags().is_empty());

    let encoded = b"Content-Type: multipart/mixed; boundary=d\r\n\r\n--d\r\nContent-Type: message/rfc822\r\nContent-Transfer-Encoding: base64\r\n\r\nU3ViamVjdDogeA0KDQpib2R5DQo=\r\n--d\r\n\r\ntail\r\n--d--\r\n";
    let message = limited(encoded, 2);
    check_nesting(&message);
    let container = message.part(1).expect("container");
    assert!(matches!(container.kind(), PartKind::Binary));
    assert_eq!(flags(container), ["limit_reached"]);
    assert!(message.root_part().flags().is_empty());
    let message = limited(encoded, 3);
    check_nesting(&message);
    assert_eq!(flagged(&message, PartFlags::LIMIT_REACHED), [0]);
}

#[test]
fn part_limit_flags_on_fixtures() {
    for file in fixtures() {
        let Some(full) = MessageParser::new().parse(&file) else {
            continue;
        };
        let total = full.parts().len();
        for max in 1..total.min(12) {
            let message = limited(&file, max);
            check_nesting(&message);
            assert_eq!(message.parts().len(), max);
            assert_eq!(flagged(&message, PartFlags::LIMIT_REACHED).len(), 1);
            for flag in [PartFlags::UNTERMINATED, PartFlags::NO_BLANK_LINE] {
                let expected = flagged(&full, flag);
                assert!(
                    flagged(&message, flag)
                        .iter()
                        .all(|id| expected.contains(id)),
                    "max_parts({max}) adds {flag:?}"
                );
            }
        }
    }
}

#[test]
fn source_of_every_message_is_constant_time() {
    let inner = b"Subject: x\r\n\r\nbody\r\n";
    let encoded = "U3ViamVjdDogeA0KDQpib2R5DQo=";
    assert_eq!(inner.len(), 20);
    let mut raw = b"Content-Type: multipart/digest; boundary=d\r\n\r\n".to_vec();
    let count = 30_000;
    for _ in 0..count {
        raw.extend_from_slice(
            b"--d\r\nContent-Type: message/rfc822\r\nContent-Transfer-Encoding: base64\r\n\r\n",
        );
        raw.extend_from_slice(encoded.as_bytes());
        raw.extend_from_slice(b"\r\n");
    }
    raw.extend_from_slice(b"--d--\r\n");
    let parser = MessageParser::new().max_parts(3 * count);
    let start = Instant::now();
    let message = parser.parse(&raw).expect("the digest parses");
    let parsing = start.elapsed();
    assert_eq!(message.messages().len(), count + 1);
    let start = Instant::now();
    let decoded = message
        .messages()
        .filter(|nested| {
            nested
                .container()
                .is_some_and(|container| nested.source() == Source::Decoded(container.id()))
        })
        .count();
    let walk = start.elapsed();
    assert_eq!(decoded, count);
    assert!(
        walk < parsing,
        "source() on {count} messages took {walk:?}, the parse {parsing:?}"
    );
}

#[test]
fn default_message_has_a_root_part() {
    let message = Message::default();
    assert_eq!(message.raw(), b"");
    assert_eq!(message.messages().len(), 1);
    assert_eq!(message.parts().len(), 1);
    let root = message.root_part();
    assert_eq!(root.id(), 0);
    assert_eq!(
        message.parts().next().map(|part| part.id()),
        Some(root.id())
    );
    assert!(matches!(root.kind(), PartKind::Text));
    assert_eq!(root.text().as_deref(), Some(""));
    assert!(root.headers().is_empty());
    assert!(root.parent().is_none());
    assert_eq!(message.text_body().len(), 0);
    assert_eq!(message.attachments().len(), 0);
    assert!(message.subject().is_none());
    assert_eq!(message.root().parts().count(), 1);
    assert_eq!(message.root().source(), Source::Raw);
}

#[test]
fn buffers_can_be_bounded() {
    let parser = MessageParser::new();
    let mut large = b"Content-Type: multipart/mixed; boundary=b\n\n".to_vec();
    for index in 0..2_000 {
        large.extend_from_slice(format!("--b\nX-Index: {index}\n\npart {index}\n").as_bytes());
    }
    large.extend_from_slice(b"--b--\n");
    let mut buffers = MessageBuffers::new();
    let message = parser.parse_with(&large, &mut buffers).expect("parses");
    assert_eq!(message.parts().len(), 2_001);
    buffers = message.into_buffers();
    buffers.shrink_to(usize::MAX);
    buffers.shrink_to(0);
    let message = parser
        .parse_with(b"Subject: small\n\nbody\n", &mut buffers)
        .expect("parses after shrinking");
    assert_eq!(message.subject(), Some("small"));
}
