/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod str;
mod table;

pub(crate) use str::{Str, validated_string};
pub(crate) use table::{Blank, Table};

use crate::{DateTime, Encoding, Greeting, Protocol, TlsVersion};
use std::{fmt, net::IpAddr};

pub(crate) const NONE: u32 = u32::MAX;
const FIRST_TEXT_ITEMS: usize = 16;
const INLINE_IDS: usize = 4;
const INLINE_PARAMS: usize = 2;
const INLINE_CONTENT_TYPES: usize = 2;
const INLINE_TEXT_ITEMS: usize = 8;
const INLINE_ADDRESS_ITEMS: usize = 4;

/// Index of a part in [`crate::Message::parts`], in document order across
/// nested messages.
pub type PartId = u32;

/// Index of a message in [`crate::Message::messages`]; 0 is the root message.
pub type MessageId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Span {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

impl Span {
    pub(crate) fn range(self) -> std::ops::Range<usize> {
        let start = self.start as usize;
        start..start + self.len as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MessageEntry {
    pub(crate) root: PartId,
    pub(crate) parts_end: PartId,
    pub(crate) source: u32,
    pub(crate) container: PartId,
    pub(crate) lists: u32,
    pub(crate) text_len: u32,
    pub(crate) html_len: u32,
    pub(crate) attachments_len: u32,
}

impl Blank for MessageEntry {
    const BLANK: MessageEntry = MessageEntry {
        root: NONE,
        parts_end: NONE,
        source: 0,
        container: NONE,
        lists: 0,
        text_len: 0,
        html_len: 0,
        attachments_len: 0,
    };
}

#[derive(Debug, Clone)]
pub(crate) struct Messages {
    root: MessageEntry,
    has_root: bool,
    nested: Vec<MessageEntry>,
}

impl Default for Messages {
    fn default() -> Self {
        Messages::EMPTY
    }
}

impl Messages {
    pub(crate) const EMPTY: Messages = Messages {
        root: MessageEntry::BLANK,
        has_root: false,
        nested: Vec::new(),
    };

    #[inline]
    pub(crate) fn len(&self) -> usize {
        usize::from(self.has_root) + self.nested.len()
    }

    #[inline]
    pub(crate) fn get(&self, index: usize) -> Option<&MessageEntry> {
        match index.checked_sub(1) {
            None => Some(&self.root),
            Some(index) => self.nested.get(index),
        }
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut MessageEntry> {
        match index.checked_sub(1) {
            None => Some(&mut self.root),
            Some(index) => self.nested.get_mut(index),
        }
    }

    pub(crate) fn first(&self) -> Option<&MessageEntry> {
        Some(&self.root)
    }

    pub(crate) fn push(&mut self, entry: MessageEntry, first: usize) -> MessageId {
        if !self.has_root {
            self.root = entry;
            self.has_root = true;
            return 0;
        }
        if self.nested.capacity() == 0 {
            self.nested.reserve_exact(first);
        }
        self.nested.push(entry);
        self.nested.len() as MessageId
    }

    pub(crate) fn pop(&mut self) -> Option<MessageEntry> {
        self.nested.pop()
    }

    pub(crate) fn clear(&mut self) {
        self.root = MessageEntry::BLANK;
        self.has_root = false;
        self.nested.clear();
    }

    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &MessageEntry> {
        std::iter::once(&self.root)
            .take(usize::from(self.has_root))
            .chain(&self.nested)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum KindTag {
    Text,
    Html,
    Binary,
    InlineBinary,
    Multipart,
    Message,
}

pub(crate) mod flag {
    pub(crate) const MISSING_DELIMITER: u16 = 1;
    pub(crate) const FALLBACK_DELIMITER: u16 = 1 << 1;
    pub(crate) const NO_BLANK_LINE: u16 = 1 << 2;
    pub(crate) const UNTERMINATED: u16 = 1 << 3;
    pub(crate) const NESTING_LIMIT: u16 = 1 << 4;
    pub(crate) const LIMIT_REACHED: u16 = 1 << 5;
    pub(crate) const MASK: u16 = 0xff;
}

pub(crate) mod role {
    pub(crate) const IN_TEXT_BODY: u16 = 1 << 8;
    pub(crate) const IN_HTML_BODY: u16 = 1 << 9;
    pub(crate) const ATTACHMENT: u16 = 1 << 10;
    pub(crate) const INLINE: u16 = 1 << 11;
    pub(crate) const COPIED: u16 = 1 << 12;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartEntry {
    pub(crate) offset_header: u32,
    pub(crate) offset_body: u32,
    pub(crate) offset_end: u32,
    pub(crate) headers: Span,
    pub(crate) parent: PartId,
    pub(crate) message: MessageId,
    pub(crate) content_type: u32,
    pub(crate) content_disposition: u32,
    pub(crate) children: Span,
    pub(crate) kind: KindTag,
    pub(crate) encoding: Encoding,
    pub(crate) flags: u16,
}

impl PartEntry {
    pub(crate) const EMPTY: PartEntry = PartEntry {
        offset_header: 0,
        offset_body: 0,
        offset_end: 0,
        headers: Span { start: 0, len: 0 },
        parent: NONE,
        message: 0,
        content_type: NONE,
        content_disposition: NONE,
        children: Span { start: 0, len: 0 },
        kind: KindTag::Text,
        encoding: Encoding::None,
        flags: 0,
    };

    pub(crate) fn has_role(&self, role: u16) -> bool {
        self.flags & role != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Value {
    #[default]
    Empty,
    Text(Str),
    TextList(Span),
    Address(Span),
    DateTime(DateTime),
    ContentType(u32),
    Received(u32),
}

pub(crate) const OTHER_NAME: u16 = u16::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaderEntry {
    pub(crate) offset_field: u32,
    pub(crate) offset_start: u32,
    pub(crate) offset_end: u32,
    pub(crate) value: Value,
    pub(crate) name: u16,
    pub(crate) other_name: u32,
}

pub(crate) const GROUP_BIT: u32 = 1 << 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AddressEntry {
    pub(crate) name: Str,
    pub(crate) address: Str,
    pub(crate) group: u32,
}

impl Blank for AddressEntry {
    const BLANK: AddressEntry = AddressEntry {
        name: Str::NONE,
        address: Str::NONE,
        group: 0,
    };
}

impl AddressEntry {
    pub(crate) fn is_group(&self) -> bool {
        self.group & GROUP_BIT != 0
    }

    pub(crate) fn members(&self) -> usize {
        (self.group & !GROUP_BIT) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParamEntry {
    pub(crate) name: Str,
    pub(crate) value: Str,
}

impl Blank for ParamEntry {
    const BLANK: ParamEntry = ParamEntry {
        name: Str::NONE,
        value: Str::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContentTypeEntry {
    pub(crate) ctype: Str,
    pub(crate) subtype: Str,
    pub(crate) params: Span,
}

impl Blank for ContentTypeEntry {
    const BLANK: ContentTypeEntry = ContentTypeEntry {
        ctype: Str::NONE,
        subtype: Str::NONE,
        params: Span { start: 0, len: 0 },
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HostEntry {
    #[default]
    None,
    Name(Str),
    Ip(IpAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReceivedEntry {
    pub(crate) from: HostEntry,
    pub(crate) from_ip: Option<IpAddr>,
    pub(crate) from_iprev: Str,
    pub(crate) by: HostEntry,
    pub(crate) for_: Str,
    pub(crate) with: Option<Protocol>,
    pub(crate) tls_version: Option<TlsVersion>,
    pub(crate) tls_cipher: Str,
    pub(crate) id: Str,
    pub(crate) ident: Str,
    pub(crate) helo: HostEntry,
    pub(crate) helo_cmd: Option<Greeting>,
    pub(crate) via: Str,
    pub(crate) date: Option<DateTime>,
}

impl ReceivedEntry {
    pub(crate) const EMPTY: ReceivedEntry = ReceivedEntry {
        from: HostEntry::None,
        from_ip: None,
        from_iprev: Str::NONE,
        by: HostEntry::None,
        for_: Str::NONE,
        with: None,
        tls_version: None,
        tls_cipher: Str::NONE,
        id: Str::NONE,
        ident: Str::NONE,
        helo: HostEntry::None,
        helo_cmd: None,
        via: Str::NONE,
        date: None,
    };
}

impl Default for ReceivedEntry {
    fn default() -> Self {
        ReceivedEntry::EMPTY
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Scratch {
    pub(crate) frames: Vec<crate::parser::Frame>,
    pub(crate) pending: Vec<PartId>,
    pub(crate) bytes: Vec<u8>,
    pub(crate) text: String,
    pub(crate) continuations: Vec<(Str, u32, Str)>,
}

impl Scratch {
    const EMPTY: Scratch = Scratch {
        frames: Vec::new(),
        pending: Vec::new(),
        bytes: Vec::new(),
        text: String::new(),
        continuations: Vec::new(),
    };

    fn clear(&mut self) {
        self.frames.clear();
        self.pending.clear();
        self.bytes.clear();
        self.text.clear();
        self.continuations.clear();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SourceBuffer {
    pub(crate) bytes: Box<[u8]>,
    pub(crate) container: PartId,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MessageData {
    pub(crate) messages: Messages,
    pub(crate) parts: Vec<PartEntry>,
    pub(crate) headers: Vec<HeaderEntry>,
    pub(crate) ids: Table<PartId, INLINE_IDS>,
    pub(crate) address_items: Table<AddressEntry, INLINE_ADDRESS_ITEMS>,
    pub(crate) params: Table<ParamEntry, INLINE_PARAMS>,
    pub(crate) content_types: Table<ContentTypeEntry, INLINE_CONTENT_TYPES>,
    pub(crate) text_items: Table<Str, INLINE_TEXT_ITEMS>,
    pub(crate) received: Vec<ReceivedEntry>,
    pub(crate) strings: String,
    pub(crate) sources: Vec<SourceBuffer>,
    pub(crate) scratch: Scratch,
}

impl MessageData {
    pub(crate) const EMPTY: MessageData = MessageData {
        messages: Messages::EMPTY,
        parts: Vec::new(),
        headers: Vec::new(),
        ids: Table::EMPTY,
        address_items: Table::EMPTY,
        params: Table::EMPTY,
        content_types: Table::EMPTY,
        text_items: Table::EMPTY,
        received: Vec::new(),
        strings: String::new(),
        sources: Vec::new(),
        scratch: Scratch::EMPTY,
    };

    pub(crate) fn empty_message() -> Self {
        let mut messages = Messages::EMPTY;
        messages.push(
            MessageEntry {
                root: 0,
                parts_end: 1,
                ..MessageEntry::BLANK
            },
            0,
        );
        MessageData {
            messages,
            parts: vec![PartEntry::EMPTY],
            ..MessageData::EMPTY
        }
    }

    fn capacity_bytes(&self) -> usize {
        [
            vec_bytes(&self.messages.nested),
            vec_bytes(&self.parts),
            vec_bytes(&self.headers),
            self.ids.capacity_bytes(),
            self.address_items.capacity_bytes(),
            self.params.capacity_bytes(),
            self.content_types.capacity_bytes(),
            self.text_items.capacity_bytes(),
            vec_bytes(&self.received),
            self.strings.capacity(),
            vec_bytes(&self.sources),
            self.sources.iter().map(|source| source.bytes.len()).sum(),
            vec_bytes(&self.scratch.frames),
            vec_bytes(&self.scratch.pending),
            self.scratch.bytes.capacity(),
            self.scratch.text.capacity(),
            vec_bytes(&self.scratch.continuations),
        ]
        .into_iter()
        .sum()
    }

    pub(crate) fn push_text_item(&mut self, item: Str) -> u32 {
        let index = self.text_items.len() as u32;
        self.text_items.push(item, FIRST_TEXT_ITEMS);
        index
    }

    pub(crate) fn clear(&mut self) {
        self.messages.clear();
        self.parts.clear();
        self.headers.clear();
        self.ids.clear();
        self.address_items.clear();
        self.params.clear();
        self.content_types.clear();
        self.text_items.clear();
        self.received.clear();
        self.strings.clear();
        self.sources.clear();
        self.scratch.clear();
    }

    pub(crate) fn source<'a>(&'a self, raw: &'a [u8], source: u32) -> &'a [u8] {
        match source.checked_sub(1) {
            None => raw,
            Some(index) => self
                .sources
                .get(index as usize)
                .map_or(&[], |source| source.bytes.as_ref()),
        }
    }

    pub(crate) fn source_container(&self, source: u32) -> Option<PartId> {
        self.sources
            .get(source.checked_sub(1)? as usize)
            .map(|source| source.container)
    }

    pub(crate) fn message_source<'a>(&'a self, raw: &'a [u8], message: MessageId) -> &'a [u8] {
        if message == 0 {
            return raw;
        }
        self.messages
            .get(message as usize)
            .map_or(raw, |entry| self.source(raw, entry.source))
    }
}

/// Reusable storage for [`crate::MessageParser::parse_with`].
///
/// A parsed message keeps its parts, headers and values in a handful of
/// vectors. Parsing many messages one after the other (an mbox or Maildir
/// import, classifier training, a batch of messages from a queue) can reuse
/// those vectors instead of allocating new ones for every message: pass the
/// buffers to `parse_with`, and when done with the message, get them back
/// with [`crate::Message::into_buffers`]. Once the vectors have grown to
/// fit the messages, parsing allocates nothing.
///
/// There is no hidden state: dropping a message instead of calling
/// `into_buffers` only means the next parse allocates, and dropping the
/// buffers frees the memory they hold.
///
/// The vectors never shrink on their own: they keep the capacity of the
/// largest message parsed with them, so one very large message in a batch
/// holds its memory for the rest of the loop. [`MessageBuffers::shrink_to`]
/// bounds that memory between two parses.
///
/// ```
/// use mail_parser::{MessageBuffers, MessageParser, mailbox::mbox::MessageIterator};
///
/// let mbox = b"From a@example.com Sat Jan  3 01:05:34 1996\n\
/// Subject: first\n\nbody\n\
/// From b@example.com Sat Jan  3 01:05:35 1996\n\
/// Subject: second\n\nbody\n";
///
/// let parser = MessageParser::new();
/// let mut buffers = MessageBuffers::new();
/// let mut subjects = Vec::new();
/// for entry in MessageIterator::new(&mbox[..]) {
///     let entry = entry.expect("readable mbox");
///     if let Some(message) = parser.parse_with(entry.contents(), &mut buffers) {
///         subjects.push(message.subject().unwrap_or_default().to_string());
///         buffers = message.into_buffers();
///     }
/// }
/// assert_eq!(subjects, ["first", "second"]);
/// ```
#[derive(Clone, Default)]
pub struct MessageBuffers {
    pub(crate) data: MessageData,
}

impl fmt::Debug for MessageBuffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageBuffers").finish_non_exhaustive()
    }
}

impl MessageBuffers {
    /// Empty buffers; they grow to fit the messages parsed with them.
    pub fn new() -> Self {
        MessageBuffers::default()
    }

    /// Frees the memory held by the buffers when it exceeds `max_bytes`,
    /// so that they hold at most `max_bytes` afterwards; the next parse then
    /// allocates again. Call it between two parses, for example after
    /// [`crate::Message::into_buffers`] in a batch loop, to keep one large
    /// message from pinning its memory. It costs nothing when the buffers
    /// are within the bound.
    ///
    /// ```
    /// use mail_parser::{MessageBuffers, MessageParser};
    ///
    /// let parser = MessageParser::new();
    /// let mut buffers = MessageBuffers::new();
    /// for raw in [&b"Subject: small\n\nbody\n"[..], b"Subject: next\n\nbody\n"] {
    ///     if let Some(message) = parser.parse_with(raw, &mut buffers) {
    ///         assert!(message.subject().is_some());
    ///         buffers = message.into_buffers();
    ///         buffers.shrink_to(1 << 20);
    ///     }
    /// }
    /// ```
    pub fn shrink_to(&mut self, max_bytes: usize) {
        if self.data.capacity_bytes() > max_bytes {
            self.data = MessageData::default();
        }
    }
}

fn vec_bytes<T>(items: &Vec<T>) -> usize {
    items.capacity() * size_of::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn root_message_stays_inline() {
        let entry = |root| MessageEntry {
            root,
            ..MessageEntry::BLANK
        };
        let mut messages = Messages::default();
        assert_eq!(messages.len(), 0);
        assert_eq!(messages.iter().count(), 0);
        assert_eq!(messages.get(0), Some(&MessageEntry::BLANK));
        assert_eq!(messages.push(entry(1), 8), 0);
        assert_eq!(messages.push(entry(2), 8), 1);
        assert_eq!(messages.push(entry(3), 8), 2);
        assert_eq!(messages.len(), 3);
        assert_eq!(
            messages.iter().map(|entry| entry.root).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        if let Some(nested) = messages.get_mut(1) {
            nested.root = 7;
        }
        assert_eq!(messages.get(1).map(|entry| entry.root), Some(7));
        assert_eq!(messages.get(3), None);
        assert_eq!(messages.pop().map(|entry| entry.root), Some(3));
        assert_eq!(messages.first().map(|entry| entry.root), Some(1));
        messages.clear();
        assert_eq!(messages.len(), 0);
        assert_eq!(messages.push(entry(4), 8), 0);
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn buffers_shrink_above_the_bound() {
        let mut raw = b"Content-Type: multipart/mixed; boundary=b\n\n".to_vec();
        for index in 0..500 {
            raw.extend_from_slice(format!("--b\nX-Index: {index}\n\npart {index}\n").as_bytes());
        }
        raw.extend_from_slice(b"--b--\n");
        let parser = crate::MessageParser::new();
        let mut buffers = MessageBuffers::new();
        assert_eq!(buffers.data.capacity_bytes(), 0);
        let message = parser.parse_with(&raw, &mut buffers).expect("parses");
        buffers = message.into_buffers();
        let held = buffers.data.capacity_bytes();
        assert!(held >= 501 * size_of::<PartEntry>());
        buffers.shrink_to(held);
        assert_eq!(buffers.data.capacity_bytes(), held);
        buffers.shrink_to(held - 1);
        assert_eq!(buffers.data.capacity_bytes(), 0);
    }

    #[test]
    fn empty_message_has_one_root_part() {
        let data = MessageData::empty_message();
        assert_eq!(data.messages.len(), 1);
        assert_eq!(data.messages.first().map(|entry| entry.root), Some(0));
        assert_eq!(data.parts, [PartEntry::EMPTY]);
        assert_eq!(data.source_container(0), None);
        assert_eq!(data.source_container(1), None);
    }

    #[test]
    fn entry_sizes() {
        assert_eq!(size_of::<Str>(), 8);
        assert_eq!(size_of::<Value>(), 12);
        assert_eq!(size_of::<MessageEntry>(), 32);
        assert_eq!(size_of::<PartEntry>(), 48);
        assert_eq!(size_of::<HeaderEntry>(), 32);
        assert_eq!(size_of::<AddressEntry>(), 20);
        assert_eq!(size_of::<ParamEntry>(), 16);
        assert_eq!(size_of::<ContentTypeEntry>(), 24);
        assert!(size_of::<ReceivedEntry>() <= 160);
        assert!(size_of::<crate::parser::Frame>() <= 96);
    }
}
