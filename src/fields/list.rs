/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{FieldCtx, push_utf8_lossy};
use crate::{decoders::charsets, store::Value};
use encodify::rfc2047::EncodedWord;
use memchr::{memchr, memchr_iter, memchr3, memmem};
use std::{iter::once, ops::Range};

impl FieldCtx<'_> {
    pub(crate) fn parse_comma_list(&mut self, value: Range<usize>) -> Value {
        let mark = self.text_items_mark();
        let range = self.trim_fws(value);
        let bytes = self.bytes(range.clone());
        if memchr3(b',', b'=', b'\n', bytes).is_none() {
            if !range.is_empty() {
                let text = self.borrow(range);
                self.push_text_item(text);
            }
        } else if memchr_iter(b'=', bytes).any(|at| bytes.get(at + 1) == Some(&b'?')) {
            ListParser::run(self, range);
        } else {
            let mut start = 0;
            for end in memchr_iter(b',', bytes).chain(once(bytes.len())) {
                self.push_plain_item(range.start + start..range.start + end);
                start = end + 1;
            }
        }
        self.text_list(mark)
    }

    fn push_plain_item(&mut self, item: Range<usize>) {
        let item = self.trim_fws(item);
        if item.is_empty() {
            return;
        }
        let bytes = self.bytes(item.clone());
        let text = if memchr(b'\n', bytes).is_none() {
            self.borrow(item)
        } else {
            self.push_with(|pool| {
                let mut lines = bytes
                    .split(|&byte| byte == b'\n')
                    .map(trim_wsp)
                    .filter(|line| !line.is_empty());
                if let Some(first) = lines.next() {
                    push_utf8_lossy(pool, first);
                }
                for line in lines {
                    pool.push(' ');
                    push_utf8_lossy(pool, line);
                }
            })
        };
        self.push_text_item(text);
    }
}

fn trim_wsp(mut text: &[u8]) -> &[u8] {
    while let [b' ' | b'\t' | b'\r', rest @ ..] = text {
        text = rest;
    }
    while let [rest @ .., b' ' | b'\t' | b'\r'] = text {
        text = rest;
    }
    text
}

struct ListParser<'a> {
    bytes: &'a [u8],
    base: usize,
    token: Option<Range<usize>>,
    is_token_start: bool,
    tokens: usize,
    single: Option<Range<usize>>,
    text: String,
    words: Vec<u8>,
    charset: &'a [u8],
    terminator: Option<Option<usize>>,
}

impl<'a> ListParser<'a> {
    fn run(ctx: &mut FieldCtx<'a>, range: Range<usize>) {
        let mut parser = ListParser {
            bytes: ctx.bytes(range.clone()),
            base: range.start,
            token: None,
            is_token_start: true,
            tokens: 0,
            single: None,
            text: ctx.take_text_scratch(),
            words: ctx.take_bytes_scratch(),
            charset: b"",
            terminator: None,
        };
        let mut pos = 0;
        while let Some(&byte) = parser.bytes.get(pos) {
            let index = pos;
            pos += 1;
            match byte {
                b'\n' => {
                    parser.add_token();
                    continue;
                }
                b' ' | b'\t' => {
                    parser.is_token_start = true;
                    continue;
                }
                b'\r' => continue,
                b',' => {
                    parser.add_item(ctx);
                    continue;
                }
                b'=' if parser.is_token_start && parser.bytes.get(pos) == Some(&b'?') => {
                    if let Some(consumed) = parser.encoded_word(index) {
                        pos = index + consumed;
                        continue;
                    }
                }
                _ => (),
            }
            parser.is_token_start = false;
            parser.token = Some(parser.token.map_or(index, |token| token.start)..index + 1);
        }
        parser.add_item(ctx);
        ctx.put_text_scratch(parser.text);
        ctx.put_bytes_scratch(parser.words);
    }

    fn push_text(&mut self, range: Range<usize>) {
        let bytes = self.bytes.get(range).unwrap_or_default();
        push_utf8_lossy(&mut self.text, bytes);
    }

    fn materialize(&mut self) {
        if let Some(single) = self.single.take() {
            self.push_text(single);
        }
    }

    fn flush_words(&mut self) {
        if !self.words.is_empty() {
            charsets::decode_append(self.charset, &self.words, &mut self.text);
            self.words.clear();
        }
    }

    fn add_token(&mut self) {
        let Some(token) = self.token.take() else {
            return;
        };
        self.is_token_start = true;
        if self.tokens == 0 {
            self.single = Some(token);
        } else {
            self.materialize();
            self.flush_words();
            self.text.push(' ');
            self.push_text(token);
        }
        self.tokens += 1;
    }

    fn add_item(&mut self, ctx: &mut FieldCtx<'_>) {
        self.add_token();
        match self.single.take() {
            Some(single) if self.tokens == 1 => {
                let text = ctx.borrow(self.base + single.start..self.base + single.end);
                ctx.push_text_item(text);
            }
            single => {
                if self.tokens > 0 {
                    if let Some(single) = single {
                        self.push_text(single);
                    }
                    self.flush_words();
                    let text = ctx.push_str(&self.text);
                    ctx.push_text_item(text);
                }
            }
        }
        self.text.clear();
        self.tokens = 0;
    }

    fn has_terminator(&mut self, from: usize) -> bool {
        match self.terminator {
            Some(Some(at)) if at >= from => return true,
            Some(None) => return false,
            _ => (),
        }
        let found = memmem::find(self.bytes.get(from..).unwrap_or_default(), b"?=")
            .map(|offset| from + offset);
        self.terminator = Some(found);
        found.is_some()
    }

    fn encoded_word(&mut self, index: usize) -> Option<usize> {
        if !self.has_terminator(index + 2) {
            return None;
        }
        let (word, consumed) = EncodedWord::parse(self.bytes.get(index..)?)?;
        let split = self.words.len();
        word.decode_append(&mut self.words).ok()?;
        let same_run =
            self.token.is_none() && split > 0 && self.charset.eq_ignore_ascii_case(word.charset);
        if !same_run {
            self.materialize();
            if let Some(pending) = self
                .words
                .get(..split)
                .filter(|pending| !pending.is_empty())
            {
                charsets::decode_append(self.charset, pending, &mut self.text);
            }
            self.words.drain(..split);
            self.charset = word.charset;
            if let Some(token) = self.token.take() {
                if self.tokens > 0 {
                    self.text.push(' ');
                }
                self.push_text(token);
                self.text.push(' ');
                self.tokens += 1;
            }
        }
        self.tokens += 1;
        Some(consumed)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        HeaderForm, HeaderValue,
        fields::{
            id::tests::{Case, Items, case_items, expected_items, items},
            tests::load_tests,
        },
    };

    const EDGE_CASES: &[Case] = &[
        (b"caf\xc3\xa9, x\xff\n", Some(&["caf\u{e9}", "x\u{fffd}"])),
        (b"a\xc3\n \xa9b\n", Some(&["a\u{fffd} \u{fffd}b"])),
    ];

    fn parse(input: &[u8]) -> Items {
        items(HeaderForm::CommaList.parse(input).value())
    }

    #[test]
    fn comma_list_fixtures() {
        let tests = load_tests("list.json");
        assert_eq!(tests.len(), 38);
        for (header, expected) in tests {
            assert_eq!(
                parse(header.as_bytes()),
                expected_items(&expected),
                "{header:?}"
            );
        }
    }

    #[test]
    fn comma_list_edge_cases() {
        for &(input, expected) in EDGE_CASES {
            assert_eq!(
                parse(input),
                case_items(expected),
                "{:?}",
                String::from_utf8_lossy(input)
            );
        }
        let parsed = HeaderForm::CommaList.parse(b" one item \r\n");
        assert!(matches!(parsed.value(), HeaderValue::TextList(list) if list.len() == 1));
    }

    #[test]
    fn comma_list_word_flood_stays_linear() {
        let mut input = b"=?ab?q?x ".repeat(50_000);
        input.extend_from_slice(b"?=\n");
        assert_eq!(parse(&input).map(|list| list.len()), Some(1));
        let mut input = b"a =?utf-8?q?".repeat(50_000);
        input.push(b'\n');
        assert_eq!(parse(&input).map(|list| list.len()), Some(1));
    }
}
