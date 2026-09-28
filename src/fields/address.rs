/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{FieldCtx, GroupMark, push_utf8_lossy};
use crate::{
    decoders::charsets,
    scan::{ByteSet, Kernel},
    store::{Str, Value},
};
use encodify::rfc2047::EncodedWord;
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
enum State {
    Name = 1,
    Address = 2,
    Quote = 4,
    Comment = 8,
}

const IN_NAME: u8 = State::Name as u8;
const IN_ADDRESS: u8 = State::Address as u8;
const IN_QUOTE: u8 = State::Quote as u8;
const IN_COMMENT: u8 = State::Comment as u8;
const EVERYWHERE: u8 = IN_NAME | IN_ADDRESS | IN_QUOTE | IN_COMMENT;

static STOPS: [u8; 256] = {
    let mut table = [0; 256];
    table[b'\n' as usize] = EVERYWHERE;
    table[b'\r' as usize] = EVERYWHERE;
    table[b' ' as usize] = EVERYWHERE;
    table[b'\t' as usize] = EVERYWHERE;
    table[b'=' as usize] = EVERYWHERE;
    table[b'"' as usize] = IN_NAME | IN_QUOTE;
    table[b'(' as usize] = IN_NAME | IN_ADDRESS | IN_COMMENT;
    table[b'\\' as usize] = IN_ADDRESS | IN_QUOTE | IN_COMMENT;
    table[b',' as usize] = IN_NAME;
    table[b'<' as usize] = IN_NAME;
    table[b'@' as usize] = IN_NAME;
    table[b':' as usize] = IN_NAME;
    table[b';' as usize] = IN_NAME;
    table[b'>' as usize] = IN_ADDRESS;
    table[b')' as usize] = IN_COMMENT;
    table
};

static SIMPLE_NAME: ByteSet = ByteSet::with_marks(b"\n\r,;<\"(:=", b"@");
static SIMPLE_QUOTED: ByteSet = ByteSet::new(b"\n\r\"\\=");
static SIMPLE_ADDRESS: ByteSet = ByteSet::new(b"\n\r>\\(=");

fn is_blank(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

fn skip_space(bytes: &[u8], mut at: usize) -> usize {
    loop {
        match bytes.get(at) {
            Some(b' ' | b'\t' | b'\r') => at += 1,
            Some(b'\n') if bytes.get(at + 1).copied().is_some_and(is_blank) => at += 2,
            _ => return at,
        }
    }
}

fn trimmed_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rposition(|&byte| !is_blank(byte))
        .map_or(0, |last| last + 1)
}

type SimpleShape = (Option<Range<usize>>, Option<Range<usize>>, usize);

#[derive(Clone, Copy)]
struct Item<'x> {
    kernel: Kernel,
    src: &'x [u8],
    start: usize,
    end: usize,
}

impl Item<'_> {
    fn bytes(&self) -> &[u8] {
        self.src.get(self.start..self.end).unwrap_or_default()
    }

    fn find(&self, set: &ByteSet, from: usize) -> Option<usize> {
        self.kernel
            .first_in_set(set, self.src, self.start + from, self.end)
            .map(|pos| pos - self.start)
    }

    fn shape(&self) -> Option<SimpleShape> {
        let bytes = self.bytes();
        let first = skip_space(bytes, 0);
        let rest = bytes.get(first..)?;
        if rest.first() == Some(&b'"') {
            let close = self.find(&SIMPLE_QUOTED, first + 1)?;
            if close == first + 1 || bytes.get(close) != Some(&b'"') {
                return None;
            }
            let open = skip_space(bytes, close + 1);
            if bytes.get(open) != Some(&b'<') {
                return None;
            }
            let (address, after) = self.angle(open + 1)?;
            return Some((Some(first + 1..close), Some(address), after));
        }
        let found = self
            .kernel
            .stop_in_set(&SIMPLE_NAME, self.src, self.start + first, self.end);
        let stop = found.at.map_or(bytes.len(), |pos| pos - self.start);
        let run = bytes.get(first..stop)?;
        let name_len = trimmed_len(run);
        let name = first..first + name_len;
        match bytes.get(stop) {
            Some(b'<') => {
                let (address, after) = self.angle(stop + 1)?;
                Some(((name_len > 0).then_some(name), Some(address), after))
            }
            None | Some(b',' | b';' | b'\r' | b'\n') if name_len > 0 => {
                if found.marked {
                    Some((None, Some(name), first + name_len))
                } else {
                    Some((Some(name), None, first + name_len))
                }
            }
            _ => None,
        }
    }

    fn angle(&self, open: usize) -> Option<(Range<usize>, usize)> {
        let close = self.angle_end(open)?;
        let content = self.bytes().get(open..close)?;
        let lead = content.iter().position(|&byte| !is_blank(byte))?;
        Some((open + lead..open + trimmed_len(content), close + 1))
    }

    fn angle_end(&self, open: usize) -> Option<usize> {
        let bytes = self.bytes();
        let mut from = open;
        loop {
            let stop = self.find(&SIMPLE_ADDRESS, from)?;
            match bytes.get(stop) {
                Some(b'>') => return Some(stop),
                Some(b'=')
                    if stop > open
                        && stop
                            .checked_sub(1)
                            .and_then(|previous| bytes.get(previous))
                            .is_some_and(|&previous| !is_blank(previous)) =>
                {
                    from = stop + 1
                }
                _ => return None,
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum List {
    Name,
    Mail,
    Comment,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Origin {
    Source,
    Decoded,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Piece {
    origin: Origin,
    start: usize,
    end: usize,
    space: bool,
}

const RECORD_LEN: usize = 16;
const TAG_SHIFT: u32 = 56;
const START_MASK: u128 = (1 << TAG_SHIFT) - 1;
const LIST_MASK: u128 = 0b11;
const DECODED_FLAG: u128 = 0b100;
const SPACE_FLAG: u128 = 0b1000;
const END_SHIFT: u32 = 64;

impl Piece {
    const EMPTY: Piece = Piece {
        origin: Origin::Source,
        start: 0,
        end: 0,
        space: false,
    };

    fn record(self, list: List) -> [u8; RECORD_LEN] {
        let mut tag = list as u128;
        if self.origin == Origin::Decoded {
            tag |= DECODED_FLAG;
        }
        if self.space {
            tag |= SPACE_FLAG;
        }
        let start = self.start as u128 & START_MASK;
        let end = (self.end as u128) << END_SHIFT;
        (start | (tag << TAG_SHIFT) | end).to_le_bytes()
    }

    fn from_record(record: &[u8; RECORD_LEN]) -> (u128, Piece) {
        let value = u128::from_le_bytes(*record);
        let tag = value >> TAG_SHIFT;
        (
            tag & LIST_MASK,
            Piece {
                origin: if tag & DECODED_FLAG != 0 {
                    Origin::Decoded
                } else {
                    Origin::Source
                },
                start: (value & START_MASK) as usize,
                end: (value >> END_SHIFT) as usize,
                space: tag & SPACE_FLAG != 0,
            },
        )
    }
}

enum PieceText<'a> {
    Text(&'a str),
    Bytes(&'a [u8]),
}

#[derive(Clone, Copy, Debug)]
struct Acc {
    count: usize,
    first: Piece,
}

impl Acc {
    const EMPTY: Acc = Acc {
        count: 0,
        first: Piece::EMPTY,
    };
}

#[derive(Clone, Copy, Debug)]
struct PendingWord<'x> {
    list: List,
    space: bool,
    charset: &'x [u8],
}

#[derive(Clone, Copy, Debug)]
struct OpenGroup {
    empty_name: Option<Str>,
    name: Option<Str>,
    marker: Option<GroupMark>,
}

struct Parser<'x> {
    src: &'x [u8],
    end: usize,
    state: State,
    outer: State,
    depth: usize,
    token: Option<(usize, usize)>,
    email: bool,
    token_start: bool,
    escaped: bool,
    last_encoded: bool,
    name: Acc,
    mail: Acc,
    comment: Acc,
    records: usize,
    raw: Vec<u8>,
    text: String,
    pending: Option<PendingWord<'x>>,
    group: Option<OpenGroup>,
}

impl FieldCtx<'_> {
    pub(crate) fn parse_address(&mut self, value: Range<usize>) -> Value {
        let mark = self.address_mark();
        let src = self.src();
        let mut parser = Parser {
            src,
            end: value.end.min(src.len()),
            state: State::Name,
            outer: State::Name,
            depth: 0,
            token: None,
            email: false,
            token_start: true,
            escaped: false,
            last_encoded: true,
            name: Acc::EMPTY,
            mail: Acc::EMPTY,
            comment: Acc::EMPTY,
            records: 0,
            raw: self.take_bytes_scratch(),
            text: self.take_text_scratch(),
            pending: None,
            group: None,
        };
        parser.run(self, value.start);
        parser.add_token();
        parser.add_address(self);
        parser.close_group(self);
        self.put_bytes_scratch(parser.raw);
        self.put_text_scratch(parser.text);
        self.address_list(mark)
    }
}

impl<'x> Parser<'x> {
    fn run(&mut self, ctx: &mut FieldCtx<'_>, start: usize) {
        let src = self.src;
        let mut pos = start;
        let mut item_start = true;
        loop {
            if item_start {
                item_start = false;
                if !self.escaped
                    && let Some(next) = self.simple_item(ctx, pos)
                {
                    pos = next;
                }
            }
            let rest = src.get(pos..self.end).unwrap_or_default();
            let mask = self.state as u8;
            let Some(run) = rest
                .iter()
                .position(|&byte| STOPS[usize::from(byte)] & mask != 0)
            else {
                if !rest.is_empty() {
                    self.extend(pos, self.end);
                }
                return;
            };
            if run > 0 {
                self.extend(pos, pos + run);
                pos += run;
            }
            let Some((&byte, after)) = rest.get(run..).and_then(<[u8]>::split_first) else {
                return;
            };
            match byte {
                b'\n' => {
                    self.add_token();
                    if !matches!(after.first(), Some(b' ' | b'\t')) {
                        return;
                    }
                    if self.state == State::Quote {
                        pos += 1;
                    } else {
                        self.token_start = true;
                        pos += 2;
                    }
                }
                b' ' | b'\t' => {
                    let blanks = 1 + after.iter().take_while(|&&byte| is_blank(byte)).count();
                    self.token_start = true;
                    self.escaped = false;
                    if self.state == State::Quote {
                        let first = self.token.map_or(pos, |(first, _)| first);
                        self.token = Some((first, pos + blanks));
                    }
                    pos += blanks;
                }
                b'\r' => pos += 1,
                b'\\' if !self.escaped => {
                    if let Some((first, _)) = self.token {
                        if self.state == State::Quote {
                            self.token = Some((first, pos));
                        }
                        self.add_token();
                    }
                    self.escaped = true;
                    pos += 1;
                }
                b'=' if self.token_start && !self.escaped && after.first() == Some(&b'?') => {
                    match self.encoded_word(pos) {
                        Some(consumed) => pos += consumed,
                        None => {
                            self.extend(pos, pos + 1);
                            pos += 1;
                        }
                    }
                }
                b',' => {
                    self.add_token();
                    self.add_address(ctx);
                    pos += 1;
                    item_start = true;
                }
                b'<' => {
                    self.email = false;
                    self.add_token();
                    self.state = State::Address;
                    pos += 1;
                }
                b'>' => {
                    self.add_token();
                    self.state = State::Name;
                    pos += 1;
                }
                b'"' if !self.escaped && self.state == State::Name => {
                    self.state = State::Quote;
                    self.add_token();
                    pos += 1;
                }
                b'"' if !self.escaped => {
                    self.add_token();
                    self.state = State::Name;
                    pos += 1;
                }
                b'@' => {
                    self.email = true;
                    self.extend(pos, pos + 1);
                    pos += 1;
                }
                b'(' if !self.escaped && self.state == State::Comment => {
                    self.depth += 1;
                    self.extend(pos, pos + 1);
                    pos += 1;
                }
                b'(' if !self.escaped => {
                    self.add_token();
                    self.outer = self.state;
                    self.state = State::Comment;
                    self.depth = 1;
                    self.last_encoded = false;
                    pos += 1;
                }
                b')' if !self.escaped && self.depth > 1 => {
                    self.depth -= 1;
                    self.extend(pos, pos + 1);
                    pos += 1;
                }
                b')' if !self.escaped => {
                    self.add_token();
                    self.depth = 0;
                    self.state = self.outer;
                    self.last_encoded = false;
                    pos += 1;
                }
                b':' if !self.escaped => {
                    self.close_group(ctx);
                    self.add_token();
                    self.open_group(ctx);
                    pos += 1;
                    item_start = true;
                }
                b';' => {
                    self.add_token();
                    self.add_address(ctx);
                    self.close_group(ctx);
                    pos += 1;
                    item_start = true;
                }
                _ => {
                    self.extend(pos, pos + 1);
                    pos += 1;
                }
            }
        }
    }

    fn extend(&mut self, start: usize, end: usize) {
        self.escaped = false;
        self.token_start = false;
        self.token = Some((self.token.map_or(start, |(first, _)| first), end));
    }

    fn acc(&self, list: List) -> &Acc {
        match list {
            List::Name => &self.name,
            List::Mail => &self.mail,
            List::Comment => &self.comment,
        }
    }

    fn acc_mut(&mut self, list: List) -> &mut Acc {
        match list {
            List::Name => &mut self.name,
            List::Mail => &mut self.mail,
            List::Comment => &mut self.comment,
        }
    }

    fn add_piece(&mut self, list: List, piece: Piece) {
        if self.acc(list).count == 0 {
            self.acc_mut(list).first = piece;
        } else {
            let at = self.records.min(self.raw.len());
            self.raw.splice(at..at, piece.record(list));
            self.records = at + RECORD_LEN;
        }
        self.acc_mut(list).count += 1;
    }

    fn add_token(&mut self) {
        let Some((start, end)) = self.token.take() else {
            return;
        };
        let (list, separated) = match self.state {
            State::Name if !self.email => (List::Name, true),
            State::Name | State::Address => (List::Mail, false),
            State::Quote => (List::Name, false),
            State::Comment => (List::Comment, true),
        };
        self.flush_word(self.raw.len());
        let space = separated && self.acc(list).count > 0;
        self.add_piece(
            list,
            Piece {
                origin: Origin::Source,
                start,
                end,
                space,
            },
        );
        self.email = false;
        self.token_start = true;
        self.escaped = false;
        self.last_encoded = false;
    }

    fn encoded_word(&mut self, at: usize) -> Option<usize> {
        let (word, consumed) = EncodedWord::parse(self.src.get(at..self.end)?)?;
        let list = match self.state {
            State::Comment => List::Comment,
            _ => List::Name,
        };
        let joinable = self.token.is_none()
            && self.last_encoded
            && self.pending.is_some_and(|pending| {
                pending.list == list && pending.charset.eq_ignore_ascii_case(word.charset)
            });
        let split = self.raw.len();
        word.decode_append(&mut self.raw).ok()?;
        if !joinable {
            self.flush_word(split);
            self.add_token();
            let space =
                !self.last_encoded && self.state != State::Quote && self.acc(list).count > 0;
            self.pending = Some(PendingWord {
                list,
                space,
                charset: word.charset,
            });
        }
        self.last_encoded = true;
        Some(consumed)
    }

    fn flush_word(&mut self, split: usize) {
        let Some(word) = self.pending.take() else {
            return;
        };
        let from = self.records.min(self.raw.len());
        let split = split.clamp(from, self.raw.len());
        let start = self.text.len();
        charsets::decode_append(
            word.charset,
            self.raw.get(from..split).unwrap_or_default(),
            &mut self.text,
        );
        let end = self.text.len();
        self.raw.drain(from..split);
        self.add_piece(
            word.list,
            Piece {
                origin: Origin::Decoded,
                start,
                end,
                space: word.space,
            },
        );
    }

    fn pieces(&self, list: List) -> impl Iterator<Item = Piece> {
        let acc = self.acc(list);
        let records: &[u8] = match acc.count {
            0 | 1 => &[],
            _ => self.raw.get(..self.records).unwrap_or_default(),
        };
        let (records, _) = records.as_chunks::<RECORD_LEN>();
        (acc.count > 0).then_some(acc.first).into_iter().chain(
            records
                .iter()
                .map(Piece::from_record)
                .filter_map(move |(id, piece)| (id == list as u128).then_some(piece)),
        )
    }

    fn piece_text(&self, piece: Piece) -> PieceText<'_> {
        match piece.origin {
            Origin::Source => {
                PieceText::Bytes(self.src.get(piece.start..piece.end).unwrap_or_default())
            }
            Origin::Decoded => {
                PieceText::Text(self.text.get(piece.start..piece.end).unwrap_or_default())
            }
        }
    }

    fn write_list(&self, out: &mut String, list: List) {
        for piece in self.pieces(list) {
            if piece.space {
                out.push(' ');
            }
            match self.piece_text(piece) {
                PieceText::Text(text) => out.push_str(text),
                PieceText::Bytes(bytes) => push_utf8_lossy(out, bytes),
            }
        }
    }

    fn has_whitespace(&self, list: List) -> bool {
        self.pieces(list).any(|piece| {
            piece.space
                || match self.piece_text(piece) {
                    PieceText::Text(text) => text.chars().any(char::is_whitespace),
                    PieceText::Bytes(bytes) => bytes
                        .utf8_chunks()
                        .any(|chunk| chunk.valid().chars().any(char::is_whitespace)),
                }
        })
    }

    fn list_str(&self, ctx: &mut FieldCtx<'_>, list: List) -> Option<Str> {
        let acc = self.acc(list);
        match acc.count {
            0 => None,
            1 => Some(match self.piece_text(acc.first) {
                PieceText::Text(text) => ctx.push_str(text),
                PieceText::Bytes(_) => ctx.borrow(acc.first.start..acc.first.end),
            }),
            _ => Some(ctx.push_with(|pool| self.write_list(pool, list))),
        }
    }

    fn with_comment(&self, ctx: &mut FieldCtx<'_>, write: impl Fn(&Self, &mut String)) -> Str {
        ctx.push_with(|pool| {
            write(self, pool);
            pool.push_str(" (");
            self.write_list(pool, List::Comment);
            pool.push(')');
        })
    }

    fn reset_lists(&mut self) {
        self.name = Acc::EMPTY;
        self.mail = Acc::EMPTY;
        self.comment = Acc::EMPTY;
        self.records = 0;
        self.raw.clear();
        self.text.clear();
    }

    fn add_address(&mut self, ctx: &mut FieldCtx<'_>) {
        self.flush_word(self.raw.len());
        let name_only = |parser: &Self, pool: &mut String| parser.write_list(pool, List::Name);
        let (name, address) = match (
            self.mail.count > 0,
            self.name.count > 0,
            self.comment.count > 0,
        ) {
            (true, true, true) => (
                Some(self.with_comment(ctx, name_only)),
                self.list_str(ctx, List::Mail),
            ),
            (true, true, false) => (
                self.list_str(ctx, List::Name),
                self.list_str(ctx, List::Mail),
            ),
            (true, false, true) => (
                self.list_str(ctx, List::Comment),
                self.list_str(ctx, List::Mail),
            ),
            (true, false, false) => (None, self.list_str(ctx, List::Mail)),
            (false, true, true) if self.has_whitespace(List::Name) => {
                (Some(self.with_comment(ctx, name_only)), None)
            }
            (false, true, true) => (
                self.list_str(ctx, List::Comment),
                self.list_str(ctx, List::Name),
            ),
            (false, true, false) => (self.list_str(ctx, List::Name), None),
            (false, false, true) => (self.list_str(ctx, List::Comment), None),
            (false, false, false) => return,
        };
        self.reset_lists();
        self.push_member(ctx, name, address);
    }

    fn push_member(&mut self, ctx: &mut FieldCtx<'_>, name: Option<Str>, address: Option<Str>) {
        if let Some(group) = self.group.as_mut()
            && group.marker.is_none()
        {
            group.marker = Some(ctx.push_group(group.name));
        }
        ctx.push_mailbox(name, address);
    }

    fn simple_item(&mut self, ctx: &mut FieldCtx<'_>, start: usize) -> Option<usize> {
        let item = Item {
            kernel: ctx.kernel(),
            src: self.src,
            start,
            end: self.end,
        };
        let bytes = item.bytes();
        let (name, address, after) = item.shape()?;
        if !matches!(
            bytes.get(skip_space(bytes, after)),
            None | Some(b',' | b';' | b'\n')
        ) {
            return None;
        }
        let name = name.map(|range| ctx.borrow(start + range.start..start + range.end));
        let address = address.map(|range| ctx.borrow(start + range.start..start + range.end));
        self.push_member(ctx, name, address);
        self.last_encoded = false;
        Some(start + after)
    }

    fn write_group_name(&self, out: &mut String) {
        self.write_list(out, List::Name);
        if self.name.count > 0 && self.mail.count > 0 {
            out.push(' ');
        }
        self.write_list(out, List::Mail);
    }

    fn open_group(&mut self, ctx: &mut FieldCtx<'_>) {
        self.flush_word(self.raw.len());
        let empty_name = match (self.name.count > 0, self.mail.count > 0) {
            (true, true) => Some(ctx.push_with(|pool| self.write_group_name(pool))),
            (true, false) => self.list_str(ctx, List::Name),
            (false, true) => self.list_str(ctx, List::Mail),
            (false, false) => None,
        };
        let name = match (empty_name, self.comment.count > 0) {
            (Some(_), true) => Some(self.with_comment(ctx, Self::write_group_name)),
            (Some(name), false) => Some(name),
            (None, true) => self.list_str(ctx, List::Comment),
            (None, false) => None,
        };
        self.reset_lists();
        self.group = Some(OpenGroup {
            empty_name,
            name,
            marker: None,
        });
    }

    fn close_group(&mut self, ctx: &mut FieldCtx<'_>) {
        match self.group.take() {
            Some(OpenGroup {
                marker: Some(marker),
                ..
            }) => ctx.end_group(marker),
            Some(OpenGroup {
                empty_name: Some(name),
                ..
            }) => {
                let marker = ctx.push_group(Some(name));
                ctx.end_group(marker);
            }
            _ => (),
        }
    }
}

/// The local part of an address (before the first `@`), when it is ASCII
/// and both sides of the `@` are non-empty.
pub fn parse_address_local_part(addr: &str) -> Option<&str> {
    let (local, domain) = addr.split_once('@')?;
    (!local.is_empty() && local.is_ascii() && !domain.is_empty()).then_some(local)
}

/// The domain of an address (after the first `@`), when the local part is
/// ASCII and both sides of the `@` are non-empty.
pub fn parse_address_domain(addr: &str) -> Option<&str> {
    let (local, domain) = addr.split_once('@')?;
    (!local.is_empty() && local.is_ascii() && !domain.is_empty()).then_some(domain)
}

/// The user of a subaddress (RFC 5233): the local part up to the first `+`.
pub fn parse_address_user_part(addr: &str) -> Option<&str> {
    let (local, domain) = addr.split_once('@')?;
    let user = local.split_once('+').map_or(local, |(user, _)| user);
    (!user.is_empty() && user.is_ascii() && !domain.is_empty()).then_some(user)
}

/// The detail of a subaddress (RFC 5233): the local part after its last `+`.
pub fn parse_address_detail_part(addr: &str) -> Option<&str> {
    let (local, domain) = addr.split_once('@')?;
    let (_, detail) = local.rsplit_once('+')?;
    (local.is_ascii() && !domain.is_empty()).then_some(detail)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_address_detail_part, parse_address_domain, parse_address_local_part,
        parse_address_user_part,
    };
    use crate::{Address, HeaderForm, HeaderValue, fields::tests::load_tests};
    use serde_json::{Value as Json, json};

    fn mailbox_json(name: Option<&str>, address: Option<&str>) -> Json {
        json!({"name": name, "address": address})
    }

    fn address_json(value: HeaderValue<'_>) -> Json {
        match value.as_address() {
            Some(list) => Json::Array(
                list.iter()
                    .map(|item| match item {
                        Address::Mailbox(mailbox) => {
                            json!({"mailbox": mailbox_json(mailbox.name(), mailbox.address())})
                        }
                        Address::Group(group) => json!({"group": {
                            "name": group.name(),
                            "mailboxes": group
                                .mailboxes()
                                .map(|mailbox| mailbox_json(mailbox.name(), mailbox.address()))
                                .collect::<Vec<_>>(),
                        }}),
                    })
                    .collect(),
            ),
            None => Json::Null,
        }
    }

    fn parse(header: &str) -> Json {
        address_json(HeaderForm::Addresses.parse(header.as_bytes()).value())
    }

    fn mailbox(name: Option<&str>, address: Option<&str>) -> Json {
        json!({"mailbox": mailbox_json(name, address)})
    }

    #[test]
    fn address_fixtures() {
        let tests = load_tests("address.json");
        assert_eq!(tests.len(), 127);
        for (header, expected) in tests {
            assert_eq!(parse(&header), expected, "{header:?}");
            let crlf = header.replace("\r\n", "\n").replace('\n', "\r\n");
            assert_eq!(parse(&crlf), expected, "{crlf:?}");
        }
    }

    #[test]
    fn issue_98_document_order() {
        let parsed = HeaderForm::Addresses.parse(b" a@b.c, g: d@e.f;, h@i.j\n");
        let list = parsed.value().as_address().expect("address list");
        assert!(list.has_groups());
        assert_eq!(
            list.mailboxes()
                .filter_map(|mailbox| mailbox.address())
                .collect::<Vec<_>>(),
            ["a@b.c", "d@e.f", "h@i.j"]
        );
        let groups: Vec<_> = list
            .groups()
            .map(|(name, run)| {
                (
                    name,
                    run.filter_map(|mailbox| mailbox.address())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            groups,
            [
                (None, vec!["a@b.c"]),
                (Some("g"), vec!["d@e.f"]),
                (None, vec!["h@i.j"])
            ]
        );
    }

    #[test]
    fn baseline_quirks_kept() {
        let parsed = HeaderForm::Addresses.parse(b"J\xf6rg <a@b.com>\n");
        assert_eq!(
            address_json(parsed.value()),
            json!([mailbox(Some("J\u{fffd}rg"), Some("a@b.com"))])
        );
    }

    #[test]
    fn simple_values_borrow() {
        for header in [
            &b"john@example.com\n"[..],
            b" John Smith <john@example.com>\r\n",
            b"\"Smith, John\" <john@example.com>,\n\tjane@example.com\n",
            b"Friends: a@b.c, <d@e.f>;\n",
            b"<mailto:list@example.com> (List)\n",
        ] {
            let parsed = HeaderForm::Addresses.parse(header);
            assert!(parsed.value().as_address().is_some());
            assert!(
                parsed
                    .data
                    .as_ref()
                    .is_some_and(|data| data.strings.is_empty()),
                "{:?}",
                String::from_utf8_lossy(header)
            );
        }
    }

    #[test]
    fn address_parts() {
        for (address, local, domain, user, detail) in [
            (
                "user@example.com",
                Some("user"),
                Some("example.com"),
                Some("user"),
                None,
            ),
            (
                "user+tag@example.com",
                Some("user+tag"),
                Some("example.com"),
                Some("user"),
                Some("tag"),
            ),
            ("a+b+c@d", Some("a+b+c"), Some("d"), Some("a"), Some("c")),
            ("a+@b", Some("a+"), Some("b"), Some("a"), Some("")),
            ("+a@b", Some("+a"), Some("b"), None, Some("a")),
            ("+@b", Some("+"), Some("b"), None, Some("")),
            ("@example.com", None, None, None, None),
            ("user@", None, None, None, None),
            ("user", None, None, None, None),
            ("", None, None, None, None),
            ("us\u{e9}r@example.com", None, None, None, None),
            (
                "user@ex\u{e1}mple.com",
                Some("user"),
                Some("ex\u{e1}mple.com"),
                Some("user"),
                None,
            ),
            ("a+\u{e9}@b", None, None, Some("a"), None),
            ("a@b+c@d", Some("a"), Some("b+c@d"), Some("a"), None),
            ("a+b@@", Some("a+b"), Some("@"), Some("a"), Some("b")),
            ("a+b", None, None, None, None),
        ] {
            assert_eq!(parse_address_local_part(address), local, "{address:?}");
            assert_eq!(parse_address_domain(address), domain, "{address:?}");
            assert_eq!(parse_address_user_part(address), user, "{address:?}");
            assert_eq!(parse_address_detail_part(address), detail, "{address:?}");
        }
    }
}
