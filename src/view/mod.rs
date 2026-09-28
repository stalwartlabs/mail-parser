/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod address;
mod content_type;
mod headers;
mod message;
mod part;
mod received;
#[cfg(feature = "serde")]
mod serialize;

pub use address::{Address, AddressList, AddressRun, Group, Mailbox};
pub use content_type::ContentType;
pub use headers::{Header, HeaderIter, HeaderValue, Headers, NamedHeaders, TextList};
pub use message::{Message, MessageRef, Source};
pub use part::{DecodeProblems, MessagePart, PartFlags, PartKind, PartRole};
pub use received::{Greeting, Host, Protocol, Received, TlsVersion};

use crate::store::{Blank, MessageData, MessageEntry, MessageId, PartEntry, PartId, Span, Str};
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Resolver<'m> {
    source: &'m [u8],
    pool: &'m str,
}

impl<'m> Resolver<'m> {
    pub(crate) fn new(source: &'m [u8], pool: &'m str) -> Self {
        Resolver { source, pool }
    }

    pub(crate) fn source(self) -> &'m [u8] {
        self.source
    }

    pub(crate) fn str(self, text: Str) -> &'m str {
        text.resolve(self.source, self.pool)
    }

    pub(crate) fn opt(self, text: Str) -> Option<&'m str> {
        text.resolve_option(self.source, self.pool)
    }

    pub(crate) fn borrow(self, range: Range<usize>) -> Option<&'m str> {
        Str::borrow(self.source, range).map(|text| self.str(text))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Ctx<'m> {
    raw: &'m [u8],
    data: &'m MessageData,
}

static EMPTY_MESSAGE: MessageEntry = MessageEntry::BLANK;
static EMPTY_PART: PartEntry = PartEntry::EMPTY;

impl<'m> Ctx<'m> {
    pub(crate) fn new(raw: &'m [u8], data: &'m MessageData) -> Self {
        Ctx { raw, data }
    }

    pub(crate) fn data(self) -> &'m MessageData {
        self.data
    }

    pub(crate) fn message_entry(self, id: MessageId) -> &'m MessageEntry {
        self.data
            .messages
            .get(id as usize)
            .unwrap_or(&EMPTY_MESSAGE)
    }

    pub(crate) fn part_entry(self, id: PartId) -> Option<&'m PartEntry> {
        self.data.parts.get(id as usize)
    }

    pub(crate) fn source_of(self, message: MessageId) -> &'m [u8] {
        self.data.message_source(self.raw, message)
    }

    #[inline]
    pub(crate) fn resolver(self, message: MessageId) -> Resolver<'m> {
        Resolver::new(self.source_of(message), &self.data.strings)
    }

    pub(crate) fn message(self, id: MessageId) -> MessageRef<'m> {
        MessageRef::new(self, id, self.message_entry(id))
    }

    pub(crate) fn part(self, id: PartId) -> Option<MessagePart<'m>> {
        self.part_entry(id)
            .map(|entry| MessagePart::new(self, id, entry))
    }

    pub(crate) fn part_or_empty(self, id: PartId) -> MessagePart<'m> {
        MessagePart::new(self, id, self.part_entry(id).unwrap_or(&EMPTY_PART))
    }

    pub(crate) fn ids(self, span: Span) -> &'m [PartId] {
        self.data.ids.get(span.range()).unwrap_or_default()
    }
}
