/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{
    MessageParser,
    classify::{Container, Disposition, Leaf, MimeClass, MimeType},
    delimiter::{Dashed, Delimiter, dashed_boundary},
};
use crate::{
    Encoding,
    scan::Kernel,
    store::{
        ContentTypeEntry, KindTag, MessageData, MessageEntry, MessageId, NONE, PartEntry, PartId,
        SourceBuffer, Span, Str, flag, role,
    },
};
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame {
    kind: FrameKind,
    part: PartId,
    message: MessageId,
    state: Container,
}

#[derive(Debug, Clone, Copy)]
enum FrameKind {
    Message(Option<Encoded>),
    Multipart {
        boundary: Str,
        closed: bool,
        children: u32,
    },
}

#[derive(Debug, Clone, Copy)]
struct Encoded {
    parent_source: u32,
    parent_base: u32,
    parent_open: u32,
    resume: Option<Delimiter>,
}

#[derive(Debug, Clone, Copy)]
struct SeekMemo {
    from: u32,
    line: u32,
}

#[derive(Debug, Clone, Copy)]
struct BoundaryMemo {
    boundary: Str,
    from: u32,
    line: u32,
}

impl SeekMemo {
    fn covers(self, src: &[u8], from: usize) -> bool {
        u32::try_from(from).is_ok_and(|start| {
            self.from <= start
                && start <= self.line
                && (start == self.from
                    || from.checked_sub(1).and_then(|last| src.get(last)) == Some(&b'\n'))
        })
    }
}

const INLINE_FRAMES: usize = 4;
const SAVED_SEEKS: usize = 4;
const SPILL_FRAMES: usize = 16;
const FIRST_MESSAGES: usize = 8;
const FIRST_SINGLE_PARTS: usize = 4;
const FIRST_MULTIPART_PARTS: usize = 16;
const FIRST_IDS: usize = 32;
const FIRST_PENDING: usize = 16;
const MIN_HEADERS: usize = 32;
const MAX_HEADERS: usize = 128;
const BYTES_PER_HEADER: usize = 128;

#[derive(Debug)]
struct Frames {
    inline: [Frame; INLINE_FRAMES],
    len: usize,
    spill: Vec<Frame>,
}

impl Frames {
    fn new(mut spill: Vec<Frame>) -> Self {
        spill.clear();
        let filler = Frame {
            kind: FrameKind::Message(None),
            part: NONE,
            message: 0,
            state: Container::message(),
        };
        Frames {
            inline: [filler; INLINE_FRAMES],
            len: 0,
            spill,
        }
    }

    fn as_slice(&self) -> &[Frame] {
        if self.spill.is_empty() {
            self.inline.get(..self.len).unwrap_or_default()
        } else {
            &self.spill
        }
    }

    fn as_mut_slice(&mut self) -> &mut [Frame] {
        if self.spill.is_empty() {
            self.inline.get_mut(..self.len).unwrap_or_default()
        } else {
            &mut self.spill
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn push(&mut self, frame: Frame) {
        if self.spill.is_empty()
            && let Some(slot) = self.inline.get_mut(self.len)
        {
            *slot = frame;
        } else {
            if self.spill.is_empty() {
                self.spill.reserve(SPILL_FRAMES);
                self.spill
                    .extend_from_slice(self.inline.get(..self.len).unwrap_or_default());
            }
            self.spill.push(frame);
        }
        self.len += 1;
    }

    fn pop(&mut self) -> Option<Frame> {
        let frame = match self.spill.pop() {
            Some(frame) => frame,
            None => *self.inline.get(self.len.checked_sub(1)?)?,
        };
        self.len -= 1;
        Some(frame)
    }

    fn get(&self, index: usize) -> Option<&Frame> {
        self.as_slice().get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Frame> {
        self.as_mut_slice().get_mut(index)
    }

    fn last(&self) -> Option<&Frame> {
        self.as_slice().last()
    }

    fn last_mut(&mut self) -> Option<&mut Frame> {
        self.as_mut_slice().last_mut()
    }

    fn into_spill(mut self) -> Vec<Frame> {
        self.spill.clear();
        self.spill
    }
}

pub(super) enum Next {
    Part(usize),
    Enter(Enter),
    SourceEnd,
}

struct LeafEnd {
    body: Range<usize>,
    encoding: Encoding,
    flags: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Enclosing {
    Continues,
    EndsHere,
}

impl Enclosing {
    fn end(self, src: &[u8], delimiter: Option<Delimiter>, body: usize) -> usize {
        delimiter.map_or(src.len(), |delimiter| match self {
            Enclosing::Continues => delimiter.content_end(src, body),
            Enclosing::EndsHere => delimiter.content_end(src, 0),
        })
    }
}

pub(super) struct Enter {
    decoded: Vec<u8>,
    part: PartId,
    resume: Option<Delimiter>,
}

pub(super) struct Builder<'p, 'd> {
    pub(super) conf: &'p MessageParser,
    pub(super) kernel: Kernel,
    pub(super) data: &'d mut MessageData,
    frames: Frames,
    pending: Vec<PartId>,
    source: u32,
    base: usize,
    open: usize,
    seek_memo: Option<SeekMemo>,
    saved_seeks: [Option<SeekMemo>; SAVED_SEEKS],
    boundary_memo: Option<BoundaryMemo>,
    encoded_depth: u32,
    max_depth: usize,
    max_parts: usize,
    max_encoded_nesting: u32,
    stopped: bool,
    headers_only: bool,
}

fn source_bytes<'a>(raw: &'a [u8], sources: &'a [SourceBuffer], source: u32) -> &'a [u8] {
    match source.checked_sub(1) {
        None => raw,
        Some(index) => sources
            .get(index as usize)
            .map_or(&[], |source| source.bytes.as_ref()),
    }
}

impl<'p, 'd> Builder<'p, 'd> {
    pub(super) fn new(
        conf: &'p MessageParser,
        data: &'d mut MessageData,
        headers_only: bool,
    ) -> Self {
        let frames = Frames::new(std::mem::take(&mut data.scratch.frames));
        let mut pending = std::mem::take(&mut data.scratch.pending);
        pending.clear();
        let (max_depth, max_parts, max_encoded_nesting) = conf.limits();
        Builder {
            conf,
            kernel: conf.scan_kernel(),
            data,
            frames,
            pending,
            source: 0,
            base: 0,
            open: 0,
            seek_memo: None,
            saved_seeks: [None; SAVED_SEEKS],
            boundary_memo: None,
            encoded_depth: 0,
            max_depth: max_depth as usize,
            max_parts: max_parts as usize,
            max_encoded_nesting,
            stopped: false,
            headers_only,
        }
    }

    pub(super) fn run(mut self, raw: &[u8]) -> bool {
        self.reserve(raw.len());
        let mut sources = std::mem::take(&mut self.data.sources);
        self.open_message(NONE, 0, None);
        let mut next = Next::Part(0);
        while !self.stopped {
            let src = source_bytes(raw, &sources, self.source);
            next = match next {
                Next::Part(offset) => self.part(src, offset),
                Next::Enter(enter) => {
                    sources.push(SourceBuffer {
                        bytes: enter.decoded.into_boxed_slice(),
                        container: enter.part,
                    });
                    self.enter(enter.part, enter.resume, sources.len() as u32)
                }
                Next::SourceEnd if self.source == 0 => break,
                Next::SourceEnd => {
                    let (resume, discard) = self.leave(src);
                    if discard {
                        sources.pop();
                    }
                    let parent = source_bytes(raw, &sources, self.source);
                    self.advance(parent, resume)
                }
            };
        }
        if self.stopped {
            self.extend_last_part(raw, &sources);
        }
        loop {
            let src = source_bytes(raw, &sources, self.source);
            if self.source == 0 {
                self.unwind(src, 0, None);
                break;
            }
            if self.leave(src).1 {
                sources.pop();
            }
        }
        let has_root = self
            .data
            .messages
            .first()
            .is_some_and(|message| message.root != NONE);
        self.data.sources = sources;
        self.pending.clear();
        self.data.scratch.frames = self.frames.into_spill();
        self.data.scratch.pending = self.pending;
        has_root
    }

    fn reserve(&mut self, len: usize) {
        self.data
            .headers
            .reserve((len / BYTES_PER_HEADER).clamp(MIN_HEADERS, MAX_HEADERS));
    }

    fn part(&mut self, src: &[u8], offset: usize) -> Next {
        let parts = self.data.parts.len();
        if parts >= self.max_parts {
            self.stopped = true;
            return Next::SourceEnd;
        }
        let headers_start = self.data.headers.len();
        let block = self.header_block(src, offset);
        let headers_len = self.data.headers.len() - headers_start;
        let body = block
            .delimiter()
            .map_or(block.body, |delimiter| delimiter.content_end(src, offset));
        let Some(frame) = self.frames.last() else {
            return Next::SourceEnd;
        };
        let message = frame.message;
        let in_multipart = matches!(frame.kind, FrameKind::Multipart { .. });
        let message_empty = self
            .data
            .messages
            .get(message as usize)
            .is_none_or(|entry| entry.root == NONE);
        if headers_len == 0
            && (block.reaches_eof() || !block.has_blank_line() && !in_multipart && message_empty)
        {
            return self.advance(src, block.delimiter());
        }

        let part_id = parts as PartId;
        let Some(frame) = self.frames.last_mut() else {
            return Next::SourceEnd;
        };
        frame.state.parts += 1;
        let parent_mime = frame.state.mime_type;
        let parent = if in_multipart {
            self.pending.push(part_id);
            frame.part
        } else {
            NONE
        };

        let mime = block.mime;
        let content_type = self
            .data
            .content_types
            .get(mime.content_type as usize)
            .copied();
        let disposition_type = self
            .data
            .content_types
            .get(mime.disposition as usize)
            .copied();
        let class = MimeClass::new(
            content_type.map(|entry| self.content_type_strs(src, entry)),
            parent_mime,
        );
        let boundary = if class.is_multipart {
            content_type.and_then(|entry| self.param(src, entry, "boundary"))
        } else {
            None
        };
        let disposition =
            Disposition::new(disposition_type.map(|entry| self.content_type_strs(src, entry).0));
        let is_text_body = class.is_text_body();
        let name = if is_text_body {
            content_type.and_then(|entry| self.param(src, entry, "name"))
        } else {
            None
        };
        let has_part_name = is_text_body
            && disposition_type
                .and_then(|entry| self.param(src, entry, "filename"))
                .or(name)
                .is_some_and(|name| !name.is_empty());

        if self.data.parts.capacity() == 0 {
            self.data.parts.reserve_exact(if class.is_multipart {
                FIRST_MULTIPART_PARTS
            } else {
                FIRST_SINGLE_PARTS
            });
        }
        self.data.parts.push(PartEntry {
            offset_header: offset as u32,
            offset_body: body as u32,
            offset_end: body as u32,
            headers: Span {
                start: headers_start as u32,
                len: headers_len as u32,
            },
            parent,
            message,
            content_type: mime.content_type,
            content_disposition: mime.disposition,
            children: Span::default(),
            kind: KindTag::Text,
            encoding: mime.encoding,
            flags: if block.has_blank_line() {
                0
            } else {
                flag::NO_BLANK_LINE
            },
        });
        if message_empty && let Some(entry) = self.data.messages.get_mut(message as usize) {
            entry.root = part_id;
        }

        if self.headers_only {
            self.update_part(part_id, |part| {
                part.kind = class.plain_kind();
                part.offset_end = src.len() as u32;
            });
            return Next::SourceEnd;
        }

        let mut class = class;
        let mut known_end = block.delimiter().map(Some);
        if let Some(boundary) = boundary {
            if block.delimiter().is_some() {
                class = class.as_broken_text();
            } else if self.frames.len() >= self.max_depth {
                self.add_flags(part_id, flag::LIMIT_REACHED);
                class = class.as_broken_text();
            } else {
                match self.open_multipart(src, boundary, class.mime_type, part_id, message, body) {
                    Ok(delimiter) => {
                        self.update_part(part_id, |part| {
                            part.kind = KindTag::Multipart;
                            if delimiter.fallback {
                                part.flags |= flag::FALLBACK_DELIMITER;
                            }
                        });
                        return self.advance(src, Some(delimiter));
                    }
                    Err(end) => {
                        class = class.as_broken_text();
                        known_end = Some(end);
                    }
                }
            }
        } else if class.mime_type == MimeType::Message
            && mime.encoding == Encoding::None
            && block.delimiter().is_none()
        {
            if self.frames.len() >= self.max_depth {
                self.add_flags(part_id, flag::LIMIT_REACHED);
            } else {
                let roles = self.frames.last().map_or(role::ATTACHMENT, |frame| {
                    frame.state.message_roles(disposition)
                });
                self.add_roles(part_id, message, roles);
                let nested = self.open_message(part_id, self.source, None);
                self.update_part(part_id, |part| {
                    part.kind = KindTag::Message;
                    part.children = Span {
                        start: nested,
                        len: 0,
                    };
                });
                return Next::Part(body);
            }
        }

        let (delimiter, found) = self.leaf_end(src, body, known_end);
        let mut flags = 0;
        if !found {
            flags |= flag::MISSING_DELIMITER;
        }
        if delimiter.is_some_and(|d| d.fallback) {
            flags |= flag::FALLBACK_DELIMITER;
        }
        let enclosing = match delimiter {
            Some(delimiter) if delimiter.frame + 1 < self.frames.len() => Enclosing::EndsHere,
            _ => Enclosing::Continues,
        };
        let end = enclosing.end(src, delimiter, body);
        let end = LeafEnd {
            body: body.min(end)..end,
            encoding: mime.encoding,
            flags,
        };
        let leaf = Leaf {
            class,
            disposition,
            has_part_name,
        };
        self.leaf(src, part_id, message, leaf, delimiter, end)
    }

    fn leaf(
        &mut self,
        src: &[u8],
        part_id: PartId,
        message: MessageId,
        leaf: Leaf,
        delimiter: Option<Delimiter>,
        end: LeafEnd,
    ) -> Next {
        if leaf.class.mime_type == MimeType::Message {
            let roles = self.frames.last().map_or(role::ATTACHMENT, |frame| {
                frame.state.message_roles(leaf.disposition)
            });
            let nesting = if end.encoding == Encoding::None || self.max_encoded_nesting == 0 {
                0
            } else if self.encoded_depth >= self.max_encoded_nesting {
                flag::NESTING_LIMIT
            } else if self.frames.len() >= self.max_depth {
                flag::LIMIT_REACHED
            } else {
                self.finish_leaf(part_id, message, &end, None, roles);
                let mut decoded = Vec::new();
                end.encoding
                    .decode_append(src.get(end.body).unwrap_or_default(), &mut decoded);
                return Next::Enter(Enter {
                    decoded,
                    part: part_id,
                    resume: delimiter,
                });
            };
            let end = LeafEnd {
                flags: end.flags | nesting,
                ..end
            };
            self.finish_leaf(part_id, message, &end, Some(KindTag::Binary), roles);
        } else if let Some(frame) = self.frames.last_mut() {
            let classified = frame.state.classify(&leaf);
            self.finish_leaf(
                part_id,
                message,
                &end,
                Some(classified.kind),
                classified.roles,
            );
        }
        self.advance(src, delimiter)
    }

    fn finish_leaf(
        &mut self,
        part: PartId,
        message: MessageId,
        end: &LeafEnd,
        kind: Option<KindTag>,
        roles: u16,
    ) {
        let Some(entry) = self.data.parts.get_mut(part as usize) else {
            return;
        };
        entry.offset_header = entry.offset_header.min(end.body.end as u32);
        entry.offset_body = end.body.start as u32;
        entry.offset_end = end.body.end as u32;
        if let Some(kind) = kind {
            entry.kind = kind;
        }
        let added = roles & !entry.flags;
        entry.flags |= end.flags | roles;
        self.count_roles(message, added);
    }

    fn enter(&mut self, part: PartId, resume: Option<Delimiter>, source: u32) -> Next {
        let encoded = Encoded {
            parent_source: self.source,
            parent_base: self.base as u32,
            parent_open: self.open as u32,
            resume,
        };
        self.boundary_memo = None;
        let parent_seek = self.seek_memo.take();
        if let Some(slot) = self.saved_seeks.get_mut(self.encoded_depth as usize) {
            *slot = parent_seek;
        }
        let nested = self.open_message(part, source, Some(encoded));
        self.update_part(part, |part| {
            part.kind = KindTag::Message;
            part.children = Span {
                start: nested,
                len: 0,
            };
        });
        self.base = self.frames.len() - 1;
        self.open = 0;
        self.source = source;
        self.encoded_depth += 1;
        Next::Part(0)
    }

    fn leave(&mut self, src: &[u8]) -> (Option<Delimiter>, bool) {
        self.unwind(src, self.base + 1, None);
        let Some(frame) = self.frames.pop() else {
            return (None, false);
        };
        let FrameKind::Message(Some(encoded)) = frame.kind else {
            return (None, false);
        };
        if let Some(root) = self
            .data
            .messages
            .get(frame.message as usize)
            .map(|entry| entry.root)
        {
            self.update_part(root, |part| part.offset_end = src.len() as u32);
        }
        let discard = self.close_message(frame.message, frame.part, true);
        self.source = encoded.parent_source;
        self.base = encoded.parent_base as usize;
        self.open = encoded.parent_open as usize;
        self.boundary_memo = None;
        self.encoded_depth = self.encoded_depth.saturating_sub(1);
        self.seek_memo = self
            .saved_seeks
            .get(self.encoded_depth as usize)
            .copied()
            .flatten();
        (encoded.resume, discard)
    }

    fn extend_last_part(&mut self, raw: &[u8], sources: &[SourceBuffer]) {
        let (Some(frame), Some(last)) = (self.frames.last().copied(), self.data.parts.last())
        else {
            return;
        };
        let last_id = (self.data.parts.len() - 1) as PartId;
        let is_open_leaf = matches!(frame.kind, FrameKind::Multipart { .. })
            && last.parent == frame.part
            && !matches!(last.kind, KindTag::Multipart | KindTag::Message);
        let holder = if is_open_leaf || frame.part == NONE {
            last_id
        } else {
            frame.part
        };
        if is_open_leaf {
            let end = source_bytes(raw, sources, self.source).len() as u32;
            self.update_part(last_id, |part| part.offset_end = end);
        }
        self.add_flags(holder, flag::LIMIT_REACHED);
    }

    pub(super) fn advance(&mut self, src: &[u8], mut delimiter: Option<Delimiter>) -> Next {
        loop {
            let Some(found) = delimiter else {
                return Next::SourceEnd;
            };
            self.unwind(src, found.frame + 1, Some(found));
            if !found.is_close {
                return Next::Part(found.next);
            }
            self.close_multipart(found.frame);
            delimiter = self.seek(src, found.next);
        }
    }

    fn open_message(
        &mut self,
        container: PartId,
        source: u32,
        encoded: Option<Encoded>,
    ) -> MessageId {
        let id = self.data.messages.push(
            MessageEntry {
                root: NONE,
                parts_end: NONE,
                source,
                container,
                lists: 0,
                text_len: 0,
                html_len: 0,
                attachments_len: 0,
            },
            FIRST_MESSAGES,
        );
        self.frames.push(Frame {
            kind: FrameKind::Message(encoded),
            part: container,
            message: id,
            state: Container::message(),
        });
        id
    }

    fn open_multipart(
        &mut self,
        src: &[u8],
        boundary: Str,
        mime_type: MimeType,
        part_id: PartId,
        message: MessageId,
        body: usize,
    ) -> Result<Delimiter, Option<Delimiter>> {
        if boundary.is_empty()
            || self.no_delimiter_ahead(src, body) && !self.boundary_occurs(src, boundary, body)
        {
            return Err(self.seek(src, body));
        }
        let Some(parent) = self.frames.last() else {
            return Err(None);
        };
        let (html_len, text_len) = self
            .data
            .messages
            .get(message as usize)
            .map_or((0, 0), |entry| (entry.html_len, entry.text_len));
        let state = parent.state.multipart(mime_type, html_len, text_len);
        if self.pending.capacity() == 0 {
            self.pending.reserve_exact(FIRST_PENDING);
        }
        self.frames.push(Frame {
            kind: FrameKind::Multipart {
                boundary,
                closed: false,
                children: self.pending.len() as u32,
            },
            part: part_id,
            message,
            state,
        });
        self.open += 1;
        self.seek_memo = None;
        let frame = self.frames.len() - 1;
        match self.seek(src, body) {
            Some(delimiter) if delimiter.frame == frame && !delimiter.is_close => Ok(delimiter),
            Some(close) if close.frame == frame => {
                self.remember_seek(body, Some(close));
                Ok(self
                    .mid_line_delimiter(src, frame, body, close.line)
                    .filter(|delimiter| !delimiter.is_close)
                    .unwrap_or(close))
            }
            other => {
                self.remember_seek(body, other);
                let limit = other.map_or(src.len(), |d| d.line);
                if let Some(delimiter) = self.mid_line_delimiter(src, frame, body, limit) {
                    return Ok(delimiter);
                }
                self.frames.pop();
                self.open -= 1;
                Err(other)
            }
        }
    }

    fn no_delimiter_ahead(&self, src: &[u8], from: usize) -> bool {
        self.seek_memo
            .is_some_and(|memo| memo.line == NONE && memo.covers(src, from))
    }

    fn remember_seek(&mut self, from: usize, found: Option<Delimiter>) {
        self.seek_memo = Some(SeekMemo {
            from: from as u32,
            line: found.map_or(NONE, |delimiter| delimiter.line as u32),
        });
    }

    fn boundary_occurs(&mut self, src: &[u8], boundary: Str, from: usize) -> bool {
        let bytes = boundary.resolve(src, &self.data.strings).as_bytes();
        let start = from as u32;
        if let Some(memo) = self.boundary_memo
            && memo.from <= start
            && start <= memo.line
            && memo.boundary.resolve(src, &self.data.strings).as_bytes() == bytes
        {
            return memo.line != NONE;
        }
        let line = match dashed_boundary(src, from, bytes) {
            Dashed::At(line) => line as u32,
            Dashed::Absent => NONE,
            Dashed::Unknown => return true,
        };
        self.boundary_memo = Some(BoundaryMemo {
            boundary,
            from: start,
            line,
        });
        line != NONE
    }

    fn close_multipart(&mut self, index: usize) {
        let Some(frame) = self.frames.get_mut(index) else {
            return;
        };
        let FrameKind::Multipart {
            closed, children, ..
        } = &mut frame.kind
        else {
            return;
        };
        if *closed {
            return;
        }
        *closed = true;
        let (part, children, message, state) = (frame.part, *children, frame.message, frame.state);
        self.open -= 1;
        self.seek_memo = None;
        if self.finish_children(part, children) == 0 {
            self.add_flags(part, flag::MISSING_DELIMITER);
        }
        if state.needs_alternative_fixup() {
            self.fix_alternative(part, message, state);
        }
    }

    fn fix_alternative(&mut self, part: PartId, message: MessageId, state: Container) {
        let Some(entry) = self.data.messages.get(message as usize).copied() else {
            return;
        };
        let (from, to) = if entry.text_len == state.text_parts && entry.html_len != state.html_parts
        {
            (role::IN_HTML_BODY, role::IN_TEXT_BODY)
        } else if entry.html_len == state.html_parts && entry.text_len != state.text_parts {
            (role::IN_TEXT_BODY, role::IN_HTML_BODY)
        } else {
            return;
        };
        let mut added = 0;
        for entry in self
            .data
            .parts
            .iter_mut()
            .skip(part as usize + 1)
            .filter(|entry| entry.message == message && entry.has_role(from) && !entry.has_role(to))
        {
            entry.flags |= to | role::COPIED;
            added += 1;
        }
        if let Some(entry) = self.data.messages.get_mut(message as usize) {
            if to == role::IN_TEXT_BODY {
                entry.text_len += added;
            } else {
                entry.html_len += added;
            }
        }
    }

    fn finish_children(&mut self, part: PartId, children: u32) -> usize {
        let start = self.data.ids.len();
        let children = (children as usize).min(self.pending.len());
        self.data
            .ids
            .extend_from_slice(self.pending.get(children..).unwrap_or_default(), FIRST_IDS);
        self.pending.truncate(children);
        let len = self.data.ids.len() - start;
        self.update_part(part, |part| {
            part.children = Span {
                start: start as u32,
                len: len as u32,
            }
        });
        len
    }

    fn unwind(&mut self, src: &[u8], keep: usize, delimiter: Option<Delimiter>) {
        while self.frames.len() > keep {
            let Some(frame) = self.frames.pop() else {
                break;
            };
            let enclosing = if self.frames.len() > keep {
                Enclosing::EndsHere
            } else {
                Enclosing::Continues
            };
            match frame.kind {
                FrameKind::Multipart {
                    closed, children, ..
                } => {
                    if !closed {
                        self.open = self.open.saturating_sub(1);
                        self.seek_memo = None;
                        self.finish_children(frame.part, children);
                        if !self.stopped {
                            self.add_flags(frame.part, flag::UNTERMINATED);
                        }
                        if frame.state.needs_alternative_fixup() {
                            self.fix_alternative(frame.part, frame.message, frame.state);
                        }
                    }
                    self.set_end(src, frame.part, delimiter, enclosing);
                }
                FrameKind::Message(_) if frame.part == NONE => {
                    if let Some(root) = self
                        .data
                        .messages
                        .get(frame.message as usize)
                        .map(|entry| entry.root)
                    {
                        self.update_part(root, |part| part.offset_end = src.len() as u32);
                    }
                    self.close_message(frame.message, NONE, false);
                }
                FrameKind::Message(_) => {
                    self.set_end(src, frame.part, delimiter, enclosing);
                    self.close_message(frame.message, frame.part, false);
                }
            }
        }
    }

    fn set_end(
        &mut self,
        src: &[u8],
        part: PartId,
        delimiter: Option<Delimiter>,
        enclosing: Enclosing,
    ) {
        self.update_part(part, |part| {
            let end = enclosing.end(src, delimiter, part.offset_body as usize) as u32;
            part.offset_header = part.offset_header.min(end);
            part.offset_body = part.offset_body.min(end);
            part.offset_end = end;
        });
    }

    fn close_message(&mut self, message: MessageId, container: PartId, encoded: bool) -> bool {
        let Some(entry) = self.data.messages.get(message as usize).copied() else {
            return false;
        };
        if entry.root != NONE {
            self.finish_lists(message, entry);
            return false;
        }
        if container == NONE {
            return false;
        }
        let problem = if self.stopped {
            flag::LIMIT_REACHED
        } else {
            flag::NO_BLANK_LINE
        };
        self.update_part(container, |part| {
            part.kind = if encoded {
                KindTag::Binary
            } else {
                KindTag::Text
            };
            part.children = Span::default();
            part.flags |= problem;
        });
        if message as usize + 1 == self.data.messages.len() {
            self.data.messages.pop();
        }
        encoded
    }

    fn finish_lists(&mut self, message: MessageId, entry: MessageEntry) {
        let text = entry.text_len as usize;
        let html = entry.html_len as usize;
        let attachments = entry.attachments_len as usize;
        let base = self.data.ids.len();
        self.data
            .ids
            .resize(base + text + html + attachments, 0, FIRST_IDS);
        let slots = self.data.ids.get_mut(base..).unwrap_or_default();
        let (text_slots, rest) = slots.split_at_mut(text.min(slots.len()));
        let (html_slots, attachment_slots) = rest.split_at_mut(html.min(rest.len()));
        let mut text_slots = text_slots.iter_mut();
        let mut html_slots = html_slots.iter_mut();
        let mut attachment_slots = attachment_slots.iter_mut();
        for (id, part) in self
            .data
            .parts
            .iter()
            .enumerate()
            .skip(entry.root as usize)
            .filter(|(_, part)| part.message == message)
        {
            let slots = [
                (role::IN_TEXT_BODY, &mut text_slots),
                (role::IN_HTML_BODY, &mut html_slots),
                (role::ATTACHMENT, &mut attachment_slots),
            ];
            for (role, slots) in slots {
                if part.has_role(role)
                    && let Some(slot) = slots.next()
                {
                    *slot = id as PartId;
                }
            }
        }
        let parts_end = self.data.parts.len() as PartId;
        if let Some(entry) = self.data.messages.get_mut(message as usize) {
            entry.lists = base as u32;
            entry.parts_end = parts_end;
        }
    }

    fn leaf_end(
        &mut self,
        src: &[u8],
        body: usize,
        known: Option<Option<Delimiter>>,
    ) -> (Option<Delimiter>, bool) {
        let found = known.unwrap_or_else(|| self.seek(src, body));
        let Some(innermost) = self.innermost_open() else {
            return (found, true);
        };
        if found.is_some_and(|d| d.frame == innermost) {
            return (found, true);
        }
        if known.is_none() {
            self.remember_seek(body, found);
        }
        let limit = found.map_or(src.len(), |d| d.line);
        match self.mid_line_delimiter(src, innermost, body, limit) {
            Some(delimiter) => (Some(delimiter), true),
            None => (found, false),
        }
    }

    fn seek(&mut self, src: &[u8], from: usize) -> Option<Delimiter> {
        if self.open == 0 {
            return None;
        }
        if let Some(memo) = self.seek_memo
            && memo.covers(src, from)
        {
            return (memo.line != NONE)
                .then(|| self.delimiter_at(src, memo.line as usize))
                .flatten();
        }
        self.scan_delimiter(src, from)
    }

    fn scan_delimiter(&self, src: &[u8], from: usize) -> Option<Delimiter> {
        if let Some(delimiter) = self.delimiter_at(src, from) {
            return Some(delimiter);
        }
        let mut cursor = from;
        while let Some(newline) = self.kernel.dash_line(src, cursor) {
            if let Some(delimiter) = self.delimiter_at(src, newline + 1) {
                return Some(delimiter);
            }
            cursor = newline + 1;
        }
        None
    }

    pub(super) fn has_open_boundaries(&self) -> bool {
        self.open > 0
    }

    pub(super) fn delimiter_at(&self, src: &[u8], line: usize) -> Option<Delimiter> {
        if !src.get(line..)?.starts_with(b"--") {
            return None;
        }
        self.open_frames()
            .find_map(|(index, frame)| match frame.kind {
                FrameKind::Multipart {
                    boundary,
                    closed: false,
                    ..
                } => Delimiter::at_line(
                    src,
                    line,
                    index,
                    boundary.resolve(src, &self.data.strings).as_bytes(),
                ),
                _ => None,
            })
    }

    fn open_frames(&self) -> impl Iterator<Item = (usize, &Frame)> {
        let base = self.base;
        self.frames
            .as_slice()
            .get(base..)
            .unwrap_or_default()
            .iter()
            .enumerate()
            .rev()
            .map(move |(index, frame)| (base + index, frame))
    }

    fn innermost_open(&self) -> Option<usize> {
        self.open_frames()
            .find(|(_, frame)| matches!(frame.kind, FrameKind::Multipart { closed: false, .. }))
            .map(|(index, _)| index)
    }

    fn mid_line_delimiter(
        &self,
        src: &[u8],
        frame: usize,
        from: usize,
        limit: usize,
    ) -> Option<Delimiter> {
        match self.frames.get(frame)?.kind {
            FrameKind::Multipart { boundary, .. } => Delimiter::mid_line(
                src,
                from,
                limit,
                frame,
                boundary.resolve(src, &self.data.strings).as_bytes(),
            ),
            FrameKind::Message(_) => None,
        }
    }

    fn content_type_strs<'a>(
        &'a self,
        src: &'a [u8],
        entry: ContentTypeEntry,
    ) -> (&'a str, Option<&'a str>) {
        (
            entry.ctype.resolve(src, &self.data.strings),
            entry.subtype.resolve_option(src, &self.data.strings),
        )
    }

    fn param(&self, src: &[u8], entry: ContentTypeEntry, name: &str) -> Option<Str> {
        let range: Range<usize> = entry.params.range();
        self.data
            .params
            .get(range)?
            .iter()
            .find(|param| param.name.resolve(src, &self.data.strings) == name)
            .map(|param| param.value)
    }

    fn update_part(&mut self, part: PartId, update: impl FnOnce(&mut PartEntry)) {
        if let Some(part) = self.data.parts.get_mut(part as usize) {
            update(part);
        }
    }

    fn add_flags(&mut self, part: PartId, flags: u16) {
        self.update_part(part, |part| part.flags |= flags);
    }

    fn add_roles(&mut self, part: PartId, message: MessageId, roles: u16) {
        let Some(entry) = self.data.parts.get_mut(part as usize) else {
            return;
        };
        let added = roles & !entry.flags;
        entry.flags |= roles;
        self.count_roles(message, added);
    }

    fn count_roles(&mut self, message: MessageId, added: u16) {
        if let Some(message) = self.data.messages.get_mut(message as usize) {
            if added & role::IN_TEXT_BODY != 0 {
                message.text_len += 1;
            }
            if added & role::IN_HTML_BODY != 0 {
                message.html_len += 1;
            }
            if added & role::ATTACHMENT != 0 {
                message.attachments_len += 1;
            }
        }
    }
}
