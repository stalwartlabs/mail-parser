/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{FieldCtx, trim_fws};
use crate::{
    decoders::charsets,
    store::{Str, Value},
};
use encodify::rfc2047::decode_text_append_with;
use memchr::{memchr2, memchr3_iter};
use std::ops::Range;

#[inline]
pub(crate) fn parse_unstructured(ctx: &mut FieldCtx<'_>, value: Range<usize>) -> Value {
    let value = ctx.trim_fws(value);
    if value.is_empty() {
        return Value::Text(Str::EMPTY);
    }
    let shape = Shape::of(ctx.bytes(value.clone()));
    Value::Text(shaped(ctx, value, shape))
}

fn shaped(ctx: &mut FieldCtx<'_>, value: Range<usize>, shape: Shape) -> Str {
    let bytes = ctx.bytes(value.clone());
    match shape {
        Shape::Line => ctx.borrow(value),
        Shape::Folded => match simdutf8::basic::from_utf8(bytes) {
            Ok(text) => ctx.push_with(|pool| unfold(text, pool)),
            Err(_) => decode(ctx, bytes),
        },
        Shape::Encoded => decode(ctx, bytes),
    }
}

pub(crate) struct Pending {
    value: Range<usize>,
    shape: Shape,
}

impl Pending {
    pub(crate) fn parse(self, ctx: &mut FieldCtx<'_>) -> Value {
        Value::Text(shaped(ctx, self.value, self.shape))
    }
}

pub(crate) fn standalone(src: &[u8], value: Range<usize>) -> Result<Value, Pending> {
    let value = trim_fws(src, value);
    if value.is_empty() {
        return Ok(Value::Text(Str::EMPTY));
    }
    let shape = Shape::of(src.get(value.clone()).unwrap_or_default());
    if shape == Shape::Line
        && let Some(text) = Str::borrow(src, value.clone())
    {
        return Ok(Value::Text(text));
    }
    Err(Pending { value, shape })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Line,
    Folded,
    Encoded,
}

impl Shape {
    fn of(bytes: &[u8]) -> Shape {
        let mut shape = Shape::Line;
        for at in memchr3_iter(b'\n', b'\r', b'=', bytes) {
            match bytes.get(at..) {
                Some([b'=', b'?', ..]) => return Shape::Encoded,
                Some([b'=', ..]) => {}
                _ => shape = Shape::Folded,
            }
        }
        shape
    }
}

#[inline(never)]
fn unfold(text: &str, out: &mut String) {
    out.reserve(text.len());
    let mut rest = text;
    while let Some((line, tail)) =
        memchr2(b'\n', b'\r', rest.as_bytes()).and_then(|at| rest.split_at_checked(at))
    {
        out.push_str(line.trim_end_matches([' ', '\t']));
        out.push(' ');
        rest = tail.trim_start_matches([' ', '\t', '\r', '\n']);
    }
    out.push_str(rest);
}

fn decode(ctx: &mut FieldCtx<'_>, bytes: &[u8]) -> Str {
    let mut scratch = ctx.take_bytes_scratch();
    let text = ctx.push_with(|pool| {
        decode_text_append_with(bytes, charsets::decode_append, &mut scratch, pool);
    });
    ctx.put_bytes_scratch(scratch);
    text
}

#[cfg(test)]
mod tests {
    use super::{Shape, parse_unstructured};
    use crate::{
        HeaderForm, MessageParser,
        decoders::charsets,
        fields::{FieldCtx, is_fws, tests::load_tests},
        store::{MessageData, Value},
    };
    use encodify::rfc2047;
    use std::path::PathBuf;

    fn parse(input: &[u8]) -> Option<String> {
        let mut data = MessageData::default();
        let mut ctx = FieldCtx::new(input, &mut data);
        let Value::Text(text) = parse_unstructured(&mut ctx, 0..input.len()) else {
            return None;
        };
        Some(ctx.resolve(text).to_string())
    }

    fn reference(input: &[u8]) -> Option<String> {
        Some(if input.iter().all(|&byte| is_fws(byte)) {
            String::new()
        } else {
            rfc2047::decode_text(input, charsets::decode_append).into_owned()
        })
    }

    #[test]
    fn fixtures() {
        let tests = load_tests("unstructured.json");
        assert_eq!(tests.len(), 54);
        for (header, expected) in tests {
            let parsed = HeaderForm::Text.parse(header.as_bytes());
            assert_eq!(parsed.value().as_text(), expected.as_str(), "{header:?}");
        }
    }

    #[test]
    fn decided_rules() {
        let cases: [(&[u8], Option<&str>); 2] = [
            (b" caf\xe9 \xff\n", Some("caf\u{fffd} \u{fffd}")),
            (b" caf\xe9\r\n \xffx\r\n", Some("caf\u{fffd} \u{fffd}x")),
        ];
        for (input, expected) in cases {
            assert_eq!(
                parse(input).as_deref(),
                expected,
                "{:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn shapes() {
        for (input, shape) in [
            (&b"plain"[..], Shape::Line),
            (b"a = b", Shape::Line),
            (b"a =", Shape::Line),
            (b"a\r\n b", Shape::Folded),
            (b"a\rb", Shape::Folded),
            (b"a\n b = c", Shape::Folded),
            (b"a =?", Shape::Encoded),
            (b"a\n b =?x", Shape::Encoded),
            (b"=?utf-8?q?a?=", Shape::Encoded),
        ] {
            assert_eq!(Shape::of(input), shape, "{input:?}");
        }
    }

    #[test]
    fn writes_the_pool_only_when_rewriting() {
        for (input, pooled) in [
            (&b" plain value\r\n"[..], false),
            (b" caf\xc3\xa9 \xe2\x82\xac\n", false),
            (b" a = b\n", false),
            (b" folded\r\n value\r\n", true),
            (b" =?utf-8?q?a?=\n", true),
            (b" caf\xe9\n", true),
        ] {
            let mut data = MessageData::default();
            let value = parse_unstructured(&mut FieldCtx::new(input, &mut data), 0..input.len());
            assert!(matches!(value, Value::Text(_)), "{input:?}");
            assert_eq!(!data.strings.is_empty(), pooled, "{input:?}");
        }
    }

    #[test]
    fn matches_decode_text_on_generated_input() {
        const FRAGMENTS: &[&[u8]] = &[
            b"a",
            b"Re:",
            b" ",
            b"  ",
            b"\t",
            b"\r",
            b"\n",
            b"\r\n",
            b"\n ",
            b"\r\n\t",
            b" \r\n ",
            b"=",
            b"?",
            b"=?",
            b"?=",
            b"_",
            b"=?utf-8?q?",
            b"=?UTF-8?B?",
            b"=?iso-8859-1?q?",
            b"=?x?q?",
            b"=?utf 8?q?",
            b"=?utf-8*en?Q?",
            b"=?*en?q?",
            b"=?\xe9?q?",
            b"=?utf-8?x?",
            b"caf=C3=A9",
            b"=E9",
            b"w6k=",
            b"w6k",
            b"=ZZ",
            b"\xc3\xa9",
            b"\xe3\x83\x8f",
            b"\xff",
            b"\xc3",
            b"\x00",
            b"*en",
            b"=?utf-8?q?caf=C3?= =?utf-8?q?=A9?=",
            b"=?utf-8?q?a?=",
            b" =?utf-8?q?x?= ",
            b"=?iso-8859-1?b?6Q==?=",
            b"=?utf-16le?b?YQBiAA==?=",
            b"=?utf-7?q?+AKM-?=",
            b"=?utf-8?b?w6k=?=\r\n =?UTF-8?Q?=C3=A9?=",
        ];
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut input = Vec::with_capacity(512);
        for _ in 0..60_000 {
            input.clear();
            let fragments = next() % 24;
            for _ in 0..fragments {
                let pick = (next() % FRAGMENTS.len() as u64) as usize;
                input.extend_from_slice(FRAGMENTS.get(pick).copied().unwrap_or_default());
            }
            assert_eq!(
                parse(&input),
                reference(&input),
                "{:?}",
                String::from_utf8_lossy(&input)
            );
        }
    }

    #[test]
    fn reuses_the_bytes_scratch() {
        let input = b" =?utf-8?q?caf=C3=A9?= =?utf-8?b?w6k=?=\n";
        let mut data = MessageData::default();
        for _ in 0..2 {
            let value = parse_unstructured(&mut FieldCtx::new(input, &mut data), 0..input.len());
            let Value::Text(text) = value else {
                panic!("text expected");
            };
            assert_eq!(text.resolve(input, &data.strings), "caf\u{e9}\u{e9}");
            assert!(data.scratch.bytes.capacity() > 0);
        }
    }

    #[test]
    fn matches_decode_text_on_every_fixture_header() {
        let eml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("eml");
        let mut suites: Vec<PathBuf> = std::fs::read_dir(&eml)
            .expect("eml directory")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        suites.sort();
        let mut values = 0;
        for suite in suites {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&suite)
                .expect("suite directory")
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "eml"))
                .collect();
            files.sort();
            for file in files {
                let raw = std::fs::read(&file).expect("fixture");
                let Some(message) = MessageParser::new().parse(&raw) else {
                    continue;
                };
                for part in message.parts() {
                    for header in part.headers().iter() {
                        let value = header.raw_value();
                        assert_eq!(
                            parse(value),
                            reference(value),
                            "{}: {:?}",
                            file.display(),
                            String::from_utf8_lossy(value)
                        );
                        values += 1;
                    }
                }
            }
        }
        assert!(values > 1_000, "{values}");
    }
}
