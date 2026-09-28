/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{FieldCtx, push_utf8_lossy as push_lossy};
use crate::{
    decoders::charsets,
    scan::{ByteSet, Kernel},
    store::{Str, Value},
};
use encodify::{Error, hex, rfc2047::EncodedWord};
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Type,
    SubType,
    AttributeName,
    AttributeValue,
    AttributeQuotedValue,
    Comment,
}

struct Parser {
    state: State,
    outer_state: State,
    comment_depth: usize,
    c_type: Option<Str>,
    c_subtype: Option<Str>,
    attr_name: Option<Str>,
    attr_charset: Option<Str>,
    attr_position: u32,
    params: u32,
    sections: usize,
    budget: u32,
    has_continued: bool,
    has_languages: bool,
    token: Option<(usize, usize)>,
    values: String,
    has_values: bool,
    is_continuation: bool,
    is_encoded_attribute: bool,
    is_escaped: bool,
    remove_crlf: bool,
    is_token_start: bool,
    apostrophes: u8,
}

fn percent_decode_append(bytes: &[u8], out: &mut Vec<u8>) -> bool {
    match hex::PERCENT.decode_append(bytes, out) {
        Ok(_) => true,
        Err(Error::Truncated { .. }) => {
            let cut = if bytes.ends_with(b"%") { 1 } else { 2 };
            bytes
                .get(..bytes.len().saturating_sub(cut))
                .is_some_and(|complete| hex::PERCENT.decode_append(complete, out).is_ok())
        }
        Err(_) => false,
    }
}

const MAX_PARAMS: u32 = 1_000;
const LANGUAGE_SUFFIX: &[u8] = b"-language";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Param,
    Language,
    First,
    Continued,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Charset,
    Plain,
    Encoded,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key(u32);

type Entry = (Str, u32, Str);

impl Key {
    const INDEX_MASK: u32 = (1 << 28) - 1;

    fn new(class: Class, index: u32, kind: Kind) -> Key {
        Key((class as u32) << 30 | index.min(Key::INDEX_MASK) << 2 | kind as u32)
    }

    fn of(entry: &Entry) -> Key {
        Key(entry.1)
    }

    fn class(self) -> Class {
        match self.0 >> 30 {
            0 => Class::Param,
            1 => Class::Language,
            2 => Class::First,
            _ => Class::Continued,
        }
    }

    fn index(self) -> u32 {
        (self.0 >> 2) & Key::INDEX_MASK
    }

    fn kind(self) -> Kind {
        match self.0 & 3 {
            0 => Kind::Charset,
            1 => Kind::Plain,
            _ => Kind::Encoded,
        }
    }
}

const MAX_SIMPLE_PARAMS: usize = 8;
const NAME: u8 = 1;
const VALUE: u8 = 2;
const QUOTED: u8 = 4;
const UPPER: u8 = 8;
const TOKEN_MARKS: &[u8] = b"!#$%&+-.^_`{|}~";
const VALUE_MARKS: &[u8] = b"*'/@:,<>[]?)";

static CLASSES: [u8; 256] = {
    let mut table = [0; 256];
    let mut byte = 0;
    while byte < 256 {
        let value = byte as u8;
        let mut class = 0;
        if value.is_ascii_alphanumeric() || value >= 0x80 {
            class = NAME | VALUE | QUOTED;
        } else if value == b' ' || value == b'\t' || value >= 0x20 && value < 0x7f {
            class = QUOTED;
        }
        if value.is_ascii_uppercase() {
            class |= UPPER;
        }
        let mut mark = 0;
        while mark < TOKEN_MARKS.len() {
            if TOKEN_MARKS[mark] == value {
                class |= NAME | VALUE;
            }
            mark += 1;
        }
        mark = 0;
        while mark < VALUE_MARKS.len() {
            if VALUE_MARKS[mark] == value {
                class |= VALUE;
            }
            mark += 1;
        }
        if value == b'"' || value == b'\\' || value == b'=' {
            class = 0;
        }
        table[byte] = class;
        byte += 1;
    }
    table
};

#[derive(Clone, Copy, Default)]
struct Token {
    start: usize,
    end: usize,
    upper: bool,
}

#[derive(Clone, Copy, Default)]
struct SimpleParam {
    name: Token,
    value: Option<(usize, usize)>,
}

struct Simple {
    ctype: Token,
    subtype: Option<Token>,
    params: [SimpleParam; MAX_SIMPLE_PARAMS],
    len: usize,
}

fn run_of(bytes: &[u8], at: usize, class: u8) -> (usize, u8) {
    let mut seen = 0;
    let end = bytes
        .get(at..)
        .unwrap_or_default()
        .iter()
        .position(|&byte| {
            let found = CLASSES[usize::from(byte)];
            seen |= found;
            found & class == 0
        })
        .map_or(bytes.len(), |len| at + len);
    (end, seen)
}

fn name_token(bytes: &[u8], at: usize) -> Option<Token> {
    let (end, seen) = run_of(bytes, at, NAME);
    (end > at).then_some(Token {
        start: at,
        end,
        upper: seen & UPPER != 0,
    })
}

static VALUE_STOPS: ByteSet = ByteSet::excluding(&CLASSES, VALUE);
static QUOTED_STOPS: ByteSet = ByteSet::excluding(&CLASSES, QUOTED);

fn skip_blank(bytes: &[u8], mut at: usize) -> usize {
    loop {
        match bytes.get(at..).unwrap_or_default() {
            [b' ' | b'\t', ..] => at += 1,
            [b'\n', b' ' | b'\t', ..] => at += 2,
            [b'\r', b'\n', b' ' | b'\t', ..] => at += 3,
            _ => return at,
        }
    }
}

fn skip_space(bytes: &[u8], mut at: usize) -> usize {
    while matches!(bytes.get(at), Some(b' ' | b'\t')) {
        at += 1;
    }
    at
}

fn at_end(bytes: &[u8], at: usize) -> bool {
    matches!(
        bytes.get(at..).unwrap_or_default(),
        [] | [b'\n', ..] | [b'\r', b'\n', ..]
    )
}

impl Kernel {
    fn value_end(self, bytes: &[u8], mut at: usize, stops: &ByteSet) -> Option<usize> {
        loop {
            let end = self
                .first_in_set(stops, bytes, at, bytes.len())
                .unwrap_or(bytes.len());
            match bytes.get(end..).unwrap_or_default() {
                [b'=', b'?', ..] => return None,
                [b'=', ..] => at = end + 1,
                _ => return Some(end),
            }
        }
    }

    fn simple_content_type(self, bytes: &[u8]) -> Option<Simple> {
        let ctype = name_token(bytes, skip_blank(bytes, 0))?;
        let mut simple = Simple {
            ctype,
            subtype: None,
            params: [SimpleParam::default(); MAX_SIMPLE_PARAMS],
            len: 0,
        };
        let mut at = ctype.end;
        if bytes.get(at) == Some(&b'/') {
            let subtype = name_token(bytes, at + 1)?;
            simple.subtype = Some(subtype);
            at = subtype.end;
        }
        loop {
            at = skip_blank(bytes, at);
            if at_end(bytes, at) {
                return Some(simple);
            }
            if bytes.get(at) != Some(&b';') {
                return None;
            }
            at = skip_blank(bytes, at + 1);
            if at_end(bytes, at) {
                return Some(simple);
            }
            let name = name_token(bytes, at)?;
            at = skip_space(bytes, name.end);
            if bytes.get(at) != Some(&b'=') {
                return None;
            }
            at = skip_space(bytes, at + 1);
            let value = if bytes.get(at) == Some(&b'"') {
                let end = self.value_end(bytes, at + 1, &QUOTED_STOPS)?;
                if bytes.get(end) != Some(&b'"') {
                    return None;
                }
                let value = (at + 1, end);
                at = end + 1;
                (end > value.0).then_some(value)
            } else {
                let end = self.value_end(bytes, at, &VALUE_STOPS)?;
                if end == at {
                    return None;
                }
                let value = (at, end);
                at = end;
                Some(value)
            };
            *simple.params.get_mut(simple.len)? = SimpleParam { name, value };
            simple.len += 1;
        }
    }
}

impl FieldCtx<'_> {
    fn parse_simple_content_type(&mut self, value: Range<usize>) -> Option<Value> {
        let base = value.start;
        let simple = self.kernel().simple_content_type(self.src().get(value)?)?;
        let lowercase = |ctx: &mut FieldCtx<'_>, token: Token| {
            ctx.lowercase_scanned(base + token.start..base + token.end, token.upper)
        };
        let ctype = lowercase(self, simple.ctype);
        let subtype = simple.subtype.map(|subtype| lowercase(self, subtype));
        let params = self.params_mark();
        for param in simple.params.get(..simple.len).unwrap_or_default() {
            let name = lowercase(self, param.name);
            if let Some((start, end)) = param.value {
                let value = self.borrow(base + start..base + end);
                self.push_param(name, value);
            }
        }
        Some(self.content_type(ctype, subtype, params))
    }

    pub(crate) fn parse_content_type(&mut self, value: Range<usize>) -> Value {
        match self.parse_simple_content_type(value.clone()) {
            Some(parsed) => parsed,
            None => self.parse_content_type_state_machine(value),
        }
    }

    fn parse_content_type_state_machine(&mut self, value: Range<usize>) -> Value {
        let src = self.src();
        let end = value.end.min(src.len());
        let mut parser = Parser {
            state: State::Type,
            outer_state: State::Type,
            comment_depth: 0,
            c_type: None,
            c_subtype: None,
            attr_name: None,
            attr_charset: None,
            attr_position: 0,
            params: self.params_mark(),
            sections: self.data.scratch.continuations.len(),
            budget: MAX_PARAMS,
            has_continued: false,
            has_languages: false,
            token: None,
            values: self.take_text_scratch(),
            has_values: false,
            is_continuation: false,
            is_encoded_attribute: false,
            is_escaped: false,
            remove_crlf: false,
            is_token_start: true,
            apostrophes: 0,
        };

        let mut pos = value.start;
        while let Some(&ch) = src.get(pos).filter(|_| pos < end) {
            let index = pos;
            pos += 1;
            match ch {
                b' ' | b'\t' => {
                    parser.is_token_start = true;
                    if parser.state == State::AttributeQuotedValue {
                        parser.extend_token(index);
                    }
                    continue;
                }
                b'\n' => {
                    let next_is_space = pos < end && matches!(src.get(pos), Some(b' ' | b'\t'));
                    match parser.state {
                        State::Type | State::AttributeName | State::SubType => {
                            parser.add_attribute(self);
                        }
                        State::AttributeValue => parser.add_value(self),
                        State::AttributeQuotedValue => {
                            if next_is_space {
                                pos += 1;
                                parser.remove_crlf = true;
                                continue;
                            }
                            parser.add_value(self);
                        }
                        State::Comment => (),
                    }
                    if next_is_space {
                        if parser.state == State::Type {
                            continue;
                        }
                        parser.state = State::AttributeName;
                        pos += 1;
                        parser.is_token_start = true;
                        continue;
                    }
                    return parser.finish(self);
                }
                b'/' if parser.state == State::Type => {
                    parser.add_attribute(self);
                    parser.state = State::SubType;
                    continue;
                }
                b';' => match parser.state {
                    State::Type | State::SubType | State::AttributeName => {
                        parser.add_attribute(self);
                        parser.state = State::AttributeName;
                        continue;
                    }
                    State::AttributeValue => {
                        if !parser.is_escaped {
                            parser.add_value(self);
                            parser.state = State::AttributeName;
                        } else {
                            parser.is_escaped = false;
                        }
                        continue;
                    }
                    _ => (),
                },
                b'*' if parser.state == State::AttributeName => {
                    if !parser.is_continuation {
                        parser.is_continuation = parser.add_attribute(self);
                    } else if !parser.is_encoded_attribute {
                        parser.add_attr_position(self);
                        parser.is_encoded_attribute = true;
                    } else {
                        parser.reset();
                    }
                    continue;
                }
                b'=' => match parser.state {
                    State::AttributeName => {
                        if !parser.is_continuation {
                            if !parser.add_attribute(self) {
                                continue;
                            }
                        } else if !parser.is_encoded_attribute {
                            parser.is_encoded_attribute = !parser.add_attr_position(self);
                        } else {
                            parser.reset();
                        }
                        parser.state = State::AttributeValue;
                        parser.apostrophes = 0;
                        continue;
                    }
                    State::AttributeValue | State::AttributeQuotedValue
                        if parser.is_token_start && src.get(pos) == Some(&b'?') =>
                    {
                        if let Some(consumed) = parser.encoded_word(self, index, end) {
                            pos = index + consumed;
                            continue;
                        }
                    }
                    _ => (),
                },
                b'"' => match parser.state {
                    State::AttributeValue => {
                        parser.is_token_start = true;
                        parser.state = State::AttributeQuotedValue;
                        continue;
                    }
                    State::AttributeQuotedValue => {
                        if !parser.is_escaped {
                            parser.add_value(self);
                            parser.state = State::AttributeName;
                            continue;
                        }
                        parser.is_escaped = false;
                    }
                    _ => continue,
                },
                b'\\' => match parser.state {
                    State::AttributeQuotedValue | State::AttributeValue => {
                        if !parser.is_escaped {
                            parser.add_partial_value(self, Some(index));
                            parser.is_escaped = true;
                            continue;
                        }
                        parser.is_escaped = false;
                    }
                    State::Comment => parser.is_escaped = !parser.is_escaped,
                    _ => continue,
                },
                b'\''
                    if parser.is_encoded_attribute
                        && !parser.is_escaped
                        && parser.apostrophes < 2
                        && matches!(
                            parser.state,
                            State::AttributeValue | State::AttributeQuotedValue
                        ) =>
                {
                    parser.add_attribute_parameter(self);
                    continue;
                }
                b'(' if parser.state != State::AttributeQuotedValue => {
                    if !parser.is_escaped {
                        match parser.state {
                            State::Type | State::AttributeName | State::SubType => {
                                parser.add_attribute(self);
                            }
                            State::AttributeValue => parser.add_value(self),
                            _ => (),
                        }
                        if parser.state == State::Comment {
                            parser.comment_depth += 1;
                        } else {
                            parser.outer_state = parser.state;
                            parser.comment_depth = 1;
                            parser.state = State::Comment;
                        }
                    } else {
                        parser.is_escaped = false;
                    }
                    continue;
                }
                b')' if parser.state == State::Comment => {
                    if !parser.is_escaped {
                        parser.comment_depth -= 1;
                        if parser.comment_depth == 0 {
                            parser.state = parser.outer_state;
                        }
                        parser.reset();
                    } else {
                        parser.is_escaped = false;
                    }
                    continue;
                }
                b'\r' => continue,
                _ => (),
            }

            parser.is_escaped = false;
            parser.is_token_start = false;
            parser.extend_token(index);
        }

        match parser.state {
            State::Type | State::AttributeName | State::SubType => {
                parser.add_attribute(self);
            }
            State::AttributeValue | State::AttributeQuotedValue => parser.add_value(self),
            State::Comment => (),
        }
        parser.finish(self)
    }
}

impl Parser {
    fn reset(&mut self) {
        self.token = None;
        self.is_token_start = true;
    }

    fn extend_token(&mut self, index: usize) {
        match &mut self.token {
            Some((_, end)) => *end = index + 1,
            None => self.token = Some((index, index + 1)),
        }
    }

    fn add_attribute(&mut self, ctx: &mut FieldCtx<'_>) -> bool {
        let Some((start, end)) = self.token else {
            return false;
        };
        let attribute = ctx.lowercase(start..end);
        match self.state {
            State::AttributeName => self.attr_name = Some(attribute),
            State::Type => self.c_type = Some(attribute),
            State::SubType => self.c_subtype = Some(attribute),
            _ => (),
        }
        self.reset();
        true
    }

    fn add_attribute_parameter(&mut self, ctx: &mut FieldCtx<'_>) {
        self.apostrophes += 1;
        let Some((start, end)) = self.token else {
            return;
        };
        let part = ctx.borrow(start..end);
        if self.attr_charset.is_none() {
            self.attr_charset = Some(part);
        } else if self.take_budget() {
            let name = ctx.language_name(self.attr_name);
            let index = self.param_count(ctx);
            ctx.push_param(name, part);
            ctx.record_continuation(
                name,
                Key::new(Class::Language, index, Kind::Plain),
                Str::NONE,
            );
            self.has_languages = true;
        }
        self.reset();
    }

    fn take_budget(&mut self) -> bool {
        let available = self.budget > 0;
        self.budget = self.budget.saturating_sub(1);
        available
    }

    fn param_count(&self, ctx: &FieldCtx<'_>) -> u32 {
        ctx.params_since(self.params).len() as u32
    }

    fn add_partial_value(&mut self, ctx: &FieldCtx<'_>, backslash: Option<usize>) {
        let Some((start, end)) = self.token else {
            return;
        };
        let in_quote = self.state == State::AttributeQuotedValue;
        let end = match backslash {
            Some(backslash) if in_quote => backslash,
            _ => end,
        };
        push_lossy(&mut self.values, ctx.bytes(start..end));
        if !in_quote {
            self.values.push(' ');
        }
        self.has_values = true;
        self.reset();
    }

    fn encoded_word(&mut self, ctx: &mut FieldCtx<'_>, at: usize, end: usize) -> Option<usize> {
        let (word, consumed) = EncodedWord::parse(ctx.bytes(at..end))?;
        let mut bytes = ctx.take_bytes_scratch();
        let decoded = word.decode_append(&mut bytes).is_ok();
        if decoded {
            self.add_partial_value(ctx, None);
            charsets::decode_append(word.charset, &bytes, &mut self.values);
            self.has_values = true;
        }
        ctx.put_bytes_scratch(bytes);
        decoded.then_some(consumed)
    }

    fn add_value(&mut self, ctx: &mut FieldCtx<'_>) {
        let Some(attr_name) = self.attr_name else {
            return;
        };
        let has_values = self.has_values;
        if self.token.is_none() && !has_values {
            return;
        }
        let value = match self.token {
            Some((start, end)) if self.remove_crlf => {
                self.remove_crlf = false;
                let mut filtered = ctx.take_bytes_scratch();
                filtered.extend(
                    ctx.bytes(start..end)
                        .iter()
                        .filter(|&&byte| byte != b'\r' && byte != b'\n'),
                );
                let text = if has_values {
                    push_lossy(&mut self.values, &filtered);
                    ctx.push_str(&self.values)
                } else {
                    ctx.push_with(|pool| push_lossy(pool, &filtered))
                };
                ctx.put_bytes_scratch(filtered);
                text
            }
            Some((start, end)) if has_values => {
                push_lossy(&mut self.values, ctx.bytes(start..end));
                ctx.push_str(&self.values)
            }
            Some((start, end)) => ctx.borrow(start..end),
            None => ctx.push_str(&self.values),
        };
        self.values.clear();
        self.has_values = false;
        self.attr_name = None;

        if !self.is_continuation {
            if self.take_budget() {
                ctx.push_param(attr_name, value);
            }
        } else {
            let encoded = std::mem::take(&mut self.is_encoded_attribute);
            let charset = self.attr_charset.take().filter(|_| encoded);
            let position = std::mem::take(&mut self.attr_position);
            self.is_continuation = false;
            if self.take_budget() {
                let (class, index) = if position > 0 {
                    self.has_continued = true;
                    (Class::Continued, position)
                } else {
                    let index = self.param_count(ctx);
                    let decoded = if encoded {
                        self.decode_extended(ctx, value, charset)
                    } else {
                        value
                    };
                    ctx.push_param(attr_name, decoded);
                    (Class::First, index)
                };
                if let Some(charset) = charset {
                    ctx.record_continuation(
                        attr_name,
                        Key::new(class, index, Kind::Charset),
                        charset,
                    );
                }
                let kind = if encoded { Kind::Encoded } else { Kind::Plain };
                ctx.record_continuation(attr_name, Key::new(class, index, kind), value);
            }
        }
        self.reset();
    }

    fn decode_extended(&mut self, ctx: &mut FieldCtx<'_>, value: Str, charset: Option<Str>) -> Str {
        let mut bytes = ctx.take_bytes_scratch();
        let decoded = percent_decode_append(ctx.resolve(value).as_bytes(), &mut bytes);
        if decoded {
            let label = charset.map_or("", |charset| ctx.resolve(charset));
            charsets::decode_append(label.as_bytes(), &bytes, &mut self.values);
        }
        ctx.put_bytes_scratch(bytes);
        let text = if decoded && self.values != ctx.resolve(value) {
            ctx.push_str(&self.values)
        } else {
            value
        };
        self.values.clear();
        text
    }

    fn add_attr_position(&mut self, ctx: &FieldCtx<'_>) -> bool {
        let Some((start, end)) = self.token else {
            return false;
        };
        self.attr_position = std::str::from_utf8(ctx.bytes(start..end))
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(0);
        self.reset();
        true
    }

    fn finish(self, ctx: &mut FieldCtx<'_>) -> Value {
        let Parser {
            c_type,
            c_subtype,
            params,
            sections,
            has_continued,
            has_languages,
            values,
            ..
        } = self;
        ctx.put_text_scratch(values);
        if has_continued || has_languages {
            ctx.assemble_params(params, sections);
        }
        ctx.data.scratch.continuations.truncate(sections);
        match c_type {
            Some(c_type) => ctx.content_type(c_type, c_subtype, params),
            None => {
                ctx.rollback_params(params);
                Value::Empty
            }
        }
    }
}

impl FieldCtx<'_> {
    fn record_continuation(&mut self, name: Str, key: Key, value: Str) {
        self.data.scratch.continuations.push((name, key.0, value));
    }

    fn language_name(&mut self, attr: Option<Str>) -> Str {
        let mut name = self.take_bytes_scratch();
        name.extend_from_slice(attr.map_or("unknown", |attr| self.resolve(attr)).as_bytes());
        name.extend_from_slice(LANGUAGE_SUFFIX);
        let text = self.push_lossy(&name);
        self.put_bytes_scratch(name);
        text
    }

    fn assemble_params(&mut self, params: u32, mark: usize) {
        let mut entries = std::mem::take(&mut self.data.scratch.continuations);
        entries.extend(
            self.params_since(params)
                .iter()
                .zip(0..)
                .map(|(param, index)| {
                    (
                        param.name,
                        Key::new(Class::Param, index, Kind::Plain).0,
                        Str::NONE,
                    )
                }),
        );
        let mut text = self.take_text_scratch();
        let mut bytes = self.take_bytes_scratch();
        let mut removed = false;
        if let Some(ours) = entries.get_mut(mark..) {
            ours.sort_unstable_by(|a, b| {
                self.resolve(a.0)
                    .cmp(self.resolve(b.0))
                    .then(a.1.cmp(&b.1))
                    .then_with(|| self.resolve(a.2).cmp(self.resolve(b.2)))
            });
            let mut rest: &[Entry] = ours;
            while let Some(first) = rest.first() {
                let name = self.resolve(first.0);
                let len = rest
                    .iter()
                    .take_while(|entry| self.resolve(entry.0) == name)
                    .count();
                let (run, tail) = rest.split_at(len);
                rest = tail;
                removed |= self.assemble_param_run(params, run, &mut text, &mut bytes);
            }
        }
        if removed {
            self.drop_removed_params(params);
        }
        self.put_text_scratch(text);
        self.put_bytes_scratch(bytes);
        self.data.scratch.continuations = entries;
    }

    fn assemble_param_run(
        &mut self,
        params: u32,
        run: &[Entry],
        text: &mut String,
        bytes: &mut Vec<u8>,
    ) -> bool {
        let first = run
            .first()
            .map(Key::of)
            .filter(|key| key.class() == Class::Param)
            .map(Key::index);
        let mut removed = false;
        for key in run.iter().map(Key::of) {
            if key.class() == Class::Language && Some(key.index()) != first {
                self.remove_param(params + key.index());
                removed = true;
            }
        }
        let Some(continued) = run
            .iter()
            .position(|entry| Key::of(entry).class() == Class::Continued)
        else {
            return removed;
        };
        let (head, continuations) = run.split_at(continued);
        let leading = head.iter().filter(|entry| {
            let key = Key::of(entry);
            key.class() == Class::First && Some(key.index()) == first
        });
        text.clear();
        if let Some(index) = first
            && leading.clone().next().is_none()
            && let Some(base) = self.params_since(params + index).first()
        {
            text.push_str(self.resolve(base.value));
        }
        let sections = leading.chain(continuations);
        let charset = sections
            .clone()
            .find(|entry| Key::of(entry).kind() == Kind::Charset)
            .map(|entry| entry.2);
        self.join_sections(sections, charset, text, bytes);
        let value = self.push_str(text);
        match first {
            Some(index) => self.set_param_value(params + index, value),
            None => {
                if let Some(&(name, ..)) = run.first() {
                    self.push_param(name, value);
                }
            }
        }
        removed
    }

    fn join_sections<'e>(
        &self,
        sections: impl Iterator<Item = &'e Entry>,
        charset: Option<Str>,
        text: &mut String,
        bytes: &mut Vec<u8>,
    ) {
        let label = charset
            .map_or("", |charset| self.resolve(charset))
            .as_bytes();
        for entry in sections {
            let raw = self.resolve(entry.2);
            match Key::of(entry).kind() {
                Kind::Charset => {}
                Kind::Encoded if percent_decode_append(raw.as_bytes(), bytes) => {}
                Kind::Encoded | Kind::Plain => {
                    flush_encoded(label, bytes, text);
                    text.push_str(raw);
                }
            }
        }
        flush_encoded(label, bytes, text);
    }
}

fn flush_encoded(label: &[u8], bytes: &mut Vec<u8>, text: &mut String) {
    if !bytes.is_empty() {
        charsets::decode_append(label, bytes, text);
        bytes.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        HeaderName, MessageParser,
        fields::{FieldCtx, tests::load_tests},
        scan::{
            Kernel,
            tests::{Rng, fixture_files},
        },
        store::{MessageData, Value},
    };

    type Parsed = Option<(String, Option<String>, Vec<(String, String)>)>;

    fn resolve(data: &MessageData, input: &[u8], value: Value) -> Parsed {
        let Value::ContentType(index) = value else {
            return None;
        };
        let entry = data.content_types.get(index as usize)?;
        let text = |text: crate::store::Str| text.resolve(input, &data.strings).to_string();
        Some((
            text(entry.ctype),
            entry
                .subtype
                .resolve_option(input, &data.strings)
                .map(str::to_string),
            data.params
                .get(entry.params.range())
                .unwrap_or_default()
                .iter()
                .map(|param| (text(param.name), text(param.value)))
                .collect(),
        ))
    }

    fn compare(input: &[u8]) -> bool {
        let mut slow = MessageData::default();
        let expected =
            FieldCtx::new(input, &mut slow).parse_content_type_state_machine(0..input.len());
        let mut accepted = false;
        for kernel in Kernel::available() {
            let mut fast = MessageData::default();
            let Some(value) = FieldCtx::with_kernel(input, &mut fast, kernel)
                .parse_simple_content_type(0..input.len())
            else {
                assert!(!accepted, "{:?}", String::from_utf8_lossy(input));
                continue;
            };
            accepted = true;
            assert_eq!(
                resolve(&fast, input, value),
                resolve(&slow, input, expected),
                "{} {:?}",
                kernel.name(),
                String::from_utf8_lossy(input)
            );
            assert_eq!(
                fast.strings,
                slow.strings,
                "{} {:?}",
                kernel.name(),
                String::from_utf8_lossy(input)
            );
        }
        accepted
    }

    #[test]
    fn simple_shapes_match_the_state_machine() {
        let mut inputs: Vec<Vec<u8>> = load_tests("content_type.json")
            .into_iter()
            .map(|(header, _)| header.into_bytes())
            .collect();
        for sample in [
            &b" text/plain; charset=us-ascii\r\n"[..],
            b" text/plain; charset=\"iso-8859-1\"\r\n",
            b" multipart/alternative;\r\n\tboundary=\"----=_NextPart_000_0001\"\r\n",
            b" TEXT/HTML; Charset=UTF-8\n",
            b" attachment; filename=\"report (1).pdf\"; size=12\r\n",
            b" inline\r\n",
            b" inline ; x=y\r\n",
            b" text/plain;\r\n",
            b" text/plain; name=\"\"\r\n",
            b" text/plain; name=\"\"; x=1\r\n",
            b" text/plain\r\n ; charset=x\r\n",
            b" text/plain\r\n charset=x\r\n",
            b" text/plain; charset =x\r\n",
            b" text/plain; charset= x\r\n",
            b" text/plain; charset=\r\n x\r\n",
            b" text/plain; boundary=Apple-Mail=_D9E8-1\r\n",
            b" text/plain; name==?utf-8?q?a?=\r\n",
            b" text/plain; name=\"=?utf-8?q?a?=\"\r\n",
            b" text/plain; name*=utf-8''a\r\n",
            b" text/plain; a=1; b=2; c=3; d=4; e=5; f=6; g=7; h=8; i=9\r\n",
            b" text/plain; charset=utf 8\r\n",
            b" text/plain (comment)\r\n",
            b" text/plain; name=caf\xc3\xa9\r\n",
            b" text/plain; name=caf\xe9\r\n",
            b" t\xe9xt/plain\r\n",
            b" text/plain; x=\"a\\\"b\"\r\n",
            b" text/plain; x=\"unterminated\r\n",
            b" text/plain\rx\r\n",
            b"text/plain",
            b"",
            b" \r\n",
            b" text/; x=y\r\n",
            b" text/plain/extra\r\n",
            b" \"text/plain\"\r\n",
            b" text/plain; x=y)z\r\n",
            b" text/plain; x=[a]@b:c,d<e>?\r\n",
            b" text/plain;; x=y\r\n",
            b" text/plain; x=y\r\n\r\nbody",
            b" text/plain; x=y\nX-Other: z\n",
            b" multipart/related; boundary=\"----=_Part_4521_1234567890.1700000000000\"; type=\"text/html\"\r\n",
            b" multipart/mixed; boundary=--_com.android.email_1234567890123456789\r\n",
            b" multipart/mixed; boundary=\"0000000000001234567890abcdef\x01tail\"\r\n",
            b" multipart/mixed; boundary=\"0000000000001234567890abcdef\\\"tail\"\r\n",
            b" multipart/mixed; boundary=0000000000001234567890abcdef(comment)\r\n",
            b" multipart/mixed; boundary=0000000000001234567890abcdef=?x?q?y?=\r\n",
        ] {
            inputs.push(sample.to_vec());
        }
        for message in fixture_files() {
            let Some(parsed) = MessageParser::new().parse(&message) else {
                continue;
            };
            for part in parsed.parts() {
                for header in part.headers().iter() {
                    if matches!(
                        header.name(),
                        HeaderName::ContentType | HeaderName::ContentDisposition
                    ) {
                        inputs.push(header.raw_value().to_vec());
                    }
                }
            }
        }
        let accepted = inputs.iter().filter(|input| compare(input)).count();
        assert!(
            accepted >= inputs.len() / 2,
            "{accepted} of {}",
            inputs.len()
        );
        let alphabet = b"aZ/;=\" \t\r\n()*'\\?_-.\xc3\xa9";
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for input in &inputs {
            for _ in 0..100 {
                let mut mutated = input.clone();
                for _ in 0..1 + rng.below(3) {
                    let at = rng.below(mutated.len() + 1);
                    let byte = alphabet
                        .get(rng.below(alphabet.len()))
                        .copied()
                        .unwrap_or(b'a');
                    if rng.below(2) == 0 && at < mutated.len() {
                        if let Some(slot) = mutated.get_mut(at) {
                            *slot = byte;
                        }
                    } else {
                        mutated.insert(at, byte);
                    }
                }
                compare(&mutated);
            }
        }
    }
}
