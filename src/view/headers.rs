/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{AddressList, ContentType, Received, Resolver};
use crate::{
    DateTime, HeaderForm, HeaderName, ParsedValue,
    header_name::{self, HeaderId, HeaderKey},
    store::{HeaderEntry, MessageData, NONE, OTHER_NAME, Str, Value},
    thread_name,
};
use std::{borrow::Cow, fmt, iter::FusedIterator, slice};

/// The header fields of one part, in the order they were written.
///
/// Accessors that return one field ([`Headers::get`], [`Headers::value`],
/// [`Headers::subject`], ...) return the last occurrence of the name, as
/// 0.11 did. Names compare ASCII case-insensitively.
///
/// ```
/// use mail_parser::{HeaderName, MessageParser};
///
/// let raw = b"Subject: first\r\nX-Tag: a\r\nSubject: second\r\n\r\nbody";
/// let message = MessageParser::new().parse(raw).expect("message");
/// let headers = message.headers();
/// assert_eq!(headers.len(), 3);
/// assert_eq!(headers.subject(), Some("second"));
/// assert_eq!(headers.all(HeaderName::Subject).count(), 2);
/// assert_eq!(headers.get("x-tag").map(|header| header.raw_value()), Some(&b" a\r\n"[..]));
/// ```
#[derive(Clone, Copy)]
pub struct Headers<'m> {
    resolver: Resolver<'m>,
    data: &'m MessageData,
    entries: &'m [HeaderEntry],
}

/// One header field: its name, its parsed value, and where it is in the
/// source buffer of its message (see [`crate::MessageRef::source`]).
#[derive(Clone, Copy)]
pub struct Header<'m> {
    resolver: Resolver<'m>,
    data: &'m MessageData,
    entry: &'m HeaderEntry,
}

/// A parsed header value. The variant follows the [`HeaderForm`] the parser
/// used for the name (see [`crate::MessageParser::header`]). Values compare
/// by content.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeaderValue<'m> {
    /// No parsable value: a value that does not parse in its form (an empty
    /// address list, date or Content-Type included), or a field parsed with
    /// [`HeaderForm::Ignore`].
    #[default]
    Empty,
    /// The trimmed field ([`HeaderForm::Raw`]) or its decoded text
    /// ([`HeaderForm::Text`]); an empty string for a field that is present
    /// but blank.
    Text(&'m str),
    /// Message identifiers ([`HeaderForm::MessageIds`]) or phrases
    /// ([`HeaderForm::CommaList`]); always a list, even with one item.
    TextList(TextList<'m>),
    /// Mailboxes and groups ([`HeaderForm::Addresses`]).
    Address(AddressList<'m>),
    /// A date ([`HeaderForm::Date`]).
    DateTime(DateTime),
    /// A Content-Type or Content-Disposition value
    /// ([`HeaderForm::ContentType`]).
    ContentType(ContentType<'m>),
    /// A Received trace field ([`HeaderForm::Received`]).
    Received(Received<'m>),
}

/// The headers with one name, returned by [`Headers::all`].
#[derive(Clone)]
pub struct NamedHeaders<'m, 'n> {
    headers: Headers<'m>,
    rest: &'m [HeaderEntry],
    name: HeaderName<'n>,
    id: u16,
}

impl<'m> Iterator for NamedHeaders<'m, '_> {
    type Item = Header<'m>;

    #[inline]
    fn next(&mut self) -> Option<Header<'m>> {
        while let Some((entry, rest)) = self.rest.split_first() {
            self.rest = rest;
            if self.headers.matches(entry, self.name.as_str(), self.id) {
                return Some(self.headers.header(entry));
            }
        }
        None
    }
}

/// Every header of a part in document order, returned by [`Headers::iter`]
/// and by iterating a [`Headers`] value.
#[derive(Clone)]
pub struct HeaderIter<'m> {
    headers: Headers<'m>,
    entries: slice::Iter<'m, HeaderEntry>,
}

impl<'m> Iterator for HeaderIter<'m> {
    type Item = Header<'m>;

    fn next(&mut self) -> Option<Header<'m>> {
        self.entries.next().map(|entry| self.headers.header(entry))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entries.size_hint()
    }
}

impl<'m> DoubleEndedIterator for HeaderIter<'m> {
    fn next_back(&mut self) -> Option<Header<'m>> {
        self.entries
            .next_back()
            .map(|entry| self.headers.header(entry))
    }
}

impl ExactSizeIterator for HeaderIter<'_> {}

impl FusedIterator for HeaderIter<'_> {}

impl<'m> IntoIterator for Headers<'m> {
    type Item = Header<'m>;
    type IntoIter = HeaderIter<'m>;

    fn into_iter(self) -> HeaderIter<'m> {
        self.iter()
    }
}

impl<'m> IntoIterator for &Headers<'m> {
    type Item = Header<'m>;
    type IntoIter = HeaderIter<'m>;

    fn into_iter(self) -> HeaderIter<'m> {
        self.iter()
    }
}

/// A list of text items: message identifiers (without angle brackets),
/// keywords, languages. Lists compare by content.
#[derive(Clone, Copy)]
pub struct TextList<'m> {
    resolver: Resolver<'m>,
    items: &'m [Str],
}

impl<'m> Headers<'m> {
    pub(crate) fn new(
        resolver: Resolver<'m>,
        data: &'m MessageData,
        entries: &'m [HeaderEntry],
    ) -> Self {
        Headers {
            resolver,
            data,
            entries,
        }
    }

    #[inline]
    fn header(&self, entry: &'m HeaderEntry) -> Header<'m> {
        Header {
            resolver: self.resolver,
            data: self.data,
            entry,
        }
    }

    /// Number of header fields.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the part has no header field.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every header in document order. `for header in headers` does the
    /// same.
    pub fn iter(&self) -> HeaderIter<'m> {
        HeaderIter {
            headers: *self,
            entries: self.entries.iter(),
        }
    }

    #[inline]
    fn matches(&self, entry: &HeaderEntry, name: &str, id: u16) -> bool {
        if id != OTHER_NAME {
            entry.name == id
        } else {
            entry.name == OTHER_NAME
                && self
                    .resolver
                    .raw_header_name(self.data, entry)
                    .eq_ignore_ascii_case(name)
        }
    }

    /// The last header with this name. A [`HeaderName::Other`] spelled
    /// like a known name finds that name.
    pub fn get<'n>(&self, name: impl Into<HeaderName<'n>>) -> Option<Header<'m>> {
        let name = name.into();
        self.find_last(name.as_str(), name.resolved_id())
    }

    /// The last header with the name of this key; see [`HeaderName::key`].
    #[inline]
    pub fn get_key(&self, key: HeaderKey<'_>) -> Option<Header<'m>> {
        self.find_last(key.as_str(), key.id())
    }

    #[inline]
    fn find_last(&self, name: &str, id: u16) -> Option<Header<'m>> {
        match id {
            OTHER_NAME => self
                .entries
                .iter()
                .rev()
                .find(|entry| self.matches(entry, name, OTHER_NAME))
                .map(|entry| self.header(entry)),
            id => self.last_known(id),
        }
    }

    #[inline]
    fn last_known(&self, id: u16) -> Option<Header<'m>> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.name == id)
            .map(|entry| self.header(entry))
    }

    #[inline]
    fn known_value(&self, id: HeaderId) -> Option<HeaderValue<'m>> {
        self.last_known(id as u16).map(|header| header.value())
    }

    /// Every header with this name, in document order. A
    /// [`HeaderName::Other`] spelled like a known name finds that name.
    #[inline]
    pub fn all<'n>(&self, name: impl Into<HeaderName<'n>>) -> NamedHeaders<'m, 'n> {
        let name = name.into();
        NamedHeaders {
            headers: *self,
            rest: self.entries,
            id: name.resolved_id(),
            name,
        }
    }

    /// Every header with the name of this key, in document order; see
    /// [`HeaderName::key`].
    #[inline]
    pub fn all_key<'k>(&self, key: HeaderKey<'k>) -> NamedHeaders<'m, 'k> {
        NamedHeaders {
            headers: *self,
            rest: self.entries,
            id: key.id(),
            name: HeaderName::Other(Cow::Borrowed(key.as_str())),
        }
    }

    /// Whether a header with this name is present.
    pub fn contains<'n>(&self, name: impl Into<HeaderName<'n>>) -> bool {
        self.get(name).is_some()
    }

    /// Whether at least one header has a known name: a cheap test that the
    /// input looks like a message rather than arbitrary text.
    pub fn has_known(&self) -> bool {
        self.entries.iter().any(|entry| entry.name != OTHER_NAME)
    }

    /// The value of the last header with this name.
    pub fn value<'n>(&self, name: impl Into<HeaderName<'n>>) -> Option<HeaderValue<'m>> {
        self.get(name).map(|header| header.value())
    }

    fn text(&self, id: HeaderId) -> Option<&'m str> {
        self.known_value(id)?.as_text()
    }

    fn address(&self, id: HeaderId) -> Option<AddressList<'m>> {
        self.known_value(id)?.as_address()
    }

    fn text_list(&self, id: HeaderId) -> Option<TextList<'m>> {
        self.known_value(id)?.as_text_list()
    }

    fn all_addresses(
        &self,
        name: HeaderName<'static>,
    ) -> impl Iterator<Item = AddressList<'m>> + use<'m> {
        self.all(name)
            .filter_map(|header| header.value().as_address())
    }

    /// The Subject field, RFC 2047 encoded words decoded.
    pub fn subject(&self) -> Option<&'m str> {
        self.text(HeaderId::Subject)
    }

    /// The Comments field, decoded.
    pub fn comments(&self) -> Option<&'m str> {
        self.text(HeaderId::Comments)
    }

    /// The MIME-Version field, as written.
    pub fn mime_version(&self) -> Option<&'m str> {
        self.text(HeaderId::MimeVersion)
    }

    /// The subject with reply and forward markers removed; see
    /// [`crate::thread_name`].
    pub fn thread_name(&self) -> Option<&'m str> {
        self.subject().map(thread_name)
    }

    /// The From field.
    pub fn from(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::From)
    }

    /// The To field.
    pub fn to(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::To)
    }

    /// The Cc field.
    pub fn cc(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::Cc)
    }

    /// The Bcc field.
    pub fn bcc(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::Bcc)
    }

    /// The Reply-To field.
    pub fn reply_to(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ReplyTo)
    }

    /// The Sender field.
    pub fn sender(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::Sender)
    }

    /// Every From field, in document order.
    pub fn all_from(&self) -> impl Iterator<Item = AddressList<'m>> + use<'m> {
        self.all_addresses(HeaderName::From)
    }

    /// Every To field, in document order.
    pub fn all_to(&self) -> impl Iterator<Item = AddressList<'m>> + use<'m> {
        self.all_addresses(HeaderName::To)
    }

    /// Every Cc field, in document order.
    pub fn all_cc(&self) -> impl Iterator<Item = AddressList<'m>> + use<'m> {
        self.all_addresses(HeaderName::Cc)
    }

    /// Every Bcc field, in document order.
    pub fn all_bcc(&self) -> impl Iterator<Item = AddressList<'m>> + use<'m> {
        self.all_addresses(HeaderName::Bcc)
    }

    /// The Resent-To field.
    pub fn resent_to(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ResentTo)
    }

    /// The Resent-From field.
    pub fn resent_from(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ResentFrom)
    }

    /// The Resent-Cc field.
    pub fn resent_cc(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ResentCc)
    }

    /// The Resent-Bcc field.
    pub fn resent_bcc(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ResentBcc)
    }

    /// The Resent-Sender field.
    pub fn resent_sender(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ResentSender)
    }

    /// The List-Archive field (RFC 2369); the URLs are in
    /// [`crate::Mailbox::address`].
    pub fn list_archive(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListArchive)
    }

    /// The List-Help field (RFC 2369); the URLs are in
    /// [`crate::Mailbox::address`].
    pub fn list_help(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListHelp)
    }

    /// The List-ID field (RFC 2919): the description in
    /// [`crate::Mailbox::name`], the identifier in [`crate::Mailbox::address`].
    pub fn list_id(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListId)
    }

    /// The List-Owner field (RFC 2369); the URLs are in
    /// [`crate::Mailbox::address`].
    pub fn list_owner(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListOwner)
    }

    /// The List-Post field (RFC 2369); the URLs are in
    /// [`crate::Mailbox::address`].
    pub fn list_post(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListPost)
    }

    /// The List-Subscribe field (RFC 2369); the URLs are in
    /// [`crate::Mailbox::address`].
    pub fn list_subscribe(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListSubscribe)
    }

    /// The List-Unsubscribe field (RFC 2369); the URLs are in
    /// [`crate::Mailbox::address`].
    pub fn list_unsubscribe(&self) -> Option<AddressList<'m>> {
        self.address(HeaderId::ListUnsubscribe)
    }

    /// The Date field.
    pub fn date(&self) -> Option<DateTime> {
        self.known_value(HeaderId::Date)?.as_datetime()
    }

    /// The Resent-Date field.
    pub fn resent_date(&self) -> Option<DateTime> {
        self.known_value(HeaderId::ResentDate)?.as_datetime()
    }

    /// The last identifier of the Message-ID field, without angle brackets.
    pub fn message_id(&self) -> Option<&'m str> {
        self.known_value(HeaderId::MessageId)?.as_text()
    }

    /// The identifiers of the In-Reply-To field.
    pub fn in_reply_to(&self) -> Option<TextList<'m>> {
        self.text_list(HeaderId::InReplyTo)
    }

    /// The identifiers of the References field.
    pub fn references(&self) -> Option<TextList<'m>> {
        self.text_list(HeaderId::References)
    }

    /// The identifiers of the Resent-Message-ID field.
    pub fn resent_message_id(&self) -> Option<TextList<'m>> {
        self.text_list(HeaderId::ResentMessageId)
    }

    /// The Return-Path field, parsed as message identifiers (the address
    /// without angle brackets).
    pub fn return_path(&self) -> Option<TextList<'m>> {
        self.text_list(HeaderId::ReturnPath)
    }

    /// The phrases of the Keywords field.
    pub fn keywords(&self) -> Option<TextList<'m>> {
        self.text_list(HeaderId::Keywords)
    }

    /// The last address of Return-Path, or else the first address of From.
    pub fn return_address(&self) -> Option<&'m str> {
        match self.known_value(HeaderId::ReturnPath) {
            Some(value @ (HeaderValue::Text(_) | HeaderValue::TextList(_))) => value.as_text(),
            _ => self.from()?.first()?.address(),
        }
    }

    /// The last Received field (the oldest hop).
    pub fn received(&self) -> Option<Received<'m>> {
        self.known_value(HeaderId::Received)?.as_received()
    }

    /// Every Received field, in document order.
    pub fn all_received(&self) -> impl Iterator<Item = Received<'m>> + use<'m> {
        self.all(HeaderName::Received)
            .filter_map(|header| header.value().as_received())
    }
}

impl<'m> Resolver<'m> {
    fn raw_header_name(self, data: &'m MessageData, entry: &HeaderEntry) -> &'m str {
        if entry.other_name != NONE {
            return data
                .text_items
                .get(entry.other_name as usize)
                .map_or("", |text| self.str(*text));
        }
        let start = entry.offset_field as usize;
        let region = self
            .source()
            .get(start..(entry.offset_start as usize).saturating_sub(1))
            .unwrap_or_default();
        let canonical = header_name::canonical(entry.name);
        if region == canonical.as_bytes() {
            return canonical;
        }
        let name = header_name::trim_blank_end(region);
        self.borrow(start..start + name.len()).unwrap_or_default()
    }
}

impl<'m> Header<'m> {
    /// The name: a known variant, or `Other` with the original spelling.
    #[inline]
    pub fn name(&self) -> HeaderName<'m> {
        match header_name::known(self.entry.name) {
            Some(known) => known.clone(),
            None => HeaderName::Other(Cow::Borrowed(self.raw_name())),
        }
    }

    /// The name as written in the message, original case kept.
    pub fn raw_name(&self) -> &'m str {
        self.resolver.raw_header_name(self.data, self.entry)
    }

    /// The value, parsed with the form the parser used for this name.
    #[inline]
    pub fn value(&self) -> HeaderValue<'m> {
        HeaderValue::new(self.resolver, self.data, self.entry.value)
    }

    /// The bytes after the colon up to the end of the field, final line
    /// break included.
    pub fn raw_value(&self) -> &'m [u8] {
        self.resolver
            .source()
            .get(self.entry.offset_start as usize..self.entry.offset_end as usize)
            .unwrap_or_default()
    }

    /// Parses the raw value again with another form, for names the parser
    /// kept raw or parsed differently.
    ///
    /// ```
    /// use mail_parser::{HeaderForm, MessageParser};
    ///
    /// let raw = b"X-Sender: Jane <jane@example.com>\r\n\r\nbody";
    /// let message = MessageParser::new().parse(raw).expect("message");
    /// let header = message.headers().get("x-sender").expect("header");
    /// let parsed = header.parse_as(HeaderForm::Addresses);
    /// let sender = parsed.value().as_address().and_then(|list| list.first());
    /// assert_eq!(sender.and_then(|mailbox| mailbox.address()), Some("jane@example.com"));
    /// ```
    pub fn parse_as(&self, form: HeaderForm) -> ParsedValue<'m> {
        ParsedValue::new(
            form,
            self.resolver.source(),
            self.entry.offset_start as usize..self.entry.offset_end as usize,
        )
    }

    /// Offset of the first byte of the name in the message's source buffer.
    pub fn offset_field(&self) -> u32 {
        self.entry.offset_field
    }

    /// Offset of the byte after the colon.
    pub fn offset_start(&self) -> u32 {
        self.entry.offset_start
    }

    /// Offset of the byte after the final line break of the field. For the
    /// last field of a header block cut short by a MIME delimiter, this is
    /// one line break past the part's body offset.
    pub fn offset_end(&self) -> u32 {
        self.entry.offset_end
    }
}

impl<'m> HeaderValue<'m> {
    #[inline]
    pub(crate) fn new(resolver: Resolver<'m>, data: &'m MessageData, value: Value) -> Self {
        match value {
            Value::Empty => HeaderValue::Empty,
            Value::Text(text) => HeaderValue::Text(resolver.str(text)),
            Value::TextList(span) => HeaderValue::TextList(TextList {
                resolver,
                items: data.text_items.get(span.range()).unwrap_or_default(),
            }),
            Value::Address(span) => HeaderValue::Address(AddressList::new(
                resolver,
                data.address_items.get(span.range()).unwrap_or_default(),
            )),
            Value::DateTime(date) => HeaderValue::DateTime(date),
            Value::ContentType(index) => data
                .content_types
                .get(index as usize)
                .map_or(HeaderValue::Empty, |entry| {
                    HeaderValue::ContentType(ContentType::new(resolver, data, entry))
                }),
            Value::Received(index) => data
                .received
                .get(index as usize)
                .map_or(HeaderValue::Empty, |entry| {
                    HeaderValue::Received(Received::new(resolver, entry))
                }),
        }
    }

    /// Whether the value is [`HeaderValue::Empty`].
    pub fn is_empty(&self) -> bool {
        matches!(self, HeaderValue::Empty)
    }

    /// The text, or the last item of a text list (the identifier of a
    /// Message-ID field, for instance).
    pub fn as_text(&self) -> Option<&'m str> {
        match self {
            HeaderValue::Text(text) => Some(text),
            HeaderValue::TextList(list) => list.last(),
            _ => None,
        }
    }

    /// The text list.
    pub fn as_text_list(&self) -> Option<TextList<'m>> {
        match self {
            HeaderValue::TextList(list) => Some(*list),
            _ => None,
        }
    }

    /// The address list.
    pub fn as_address(&self) -> Option<AddressList<'m>> {
        match self {
            HeaderValue::Address(list) => Some(*list),
            _ => None,
        }
    }

    /// The date.
    pub fn as_datetime(&self) -> Option<DateTime> {
        match self {
            HeaderValue::DateTime(date) => Some(*date),
            _ => None,
        }
    }

    /// The Content-Type or Content-Disposition value.
    pub fn as_content_type(&self) -> Option<ContentType<'m>> {
        match self {
            HeaderValue::ContentType(content_type) => Some(*content_type),
            _ => None,
        }
    }

    /// The Received trace field.
    pub fn as_received(&self) -> Option<Received<'m>> {
        match self {
            HeaderValue::Received(received) => Some(*received),
            _ => None,
        }
    }
}

impl<'m> TextList<'m> {
    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list has no item.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The item at `index`.
    pub fn get(&self, index: usize) -> Option<&'m str> {
        self.items.get(index).map(|item| self.resolver.str(*item))
    }

    /// The first item.
    pub fn first(&self) -> Option<&'m str> {
        self.items.first().map(|item| self.resolver.str(*item))
    }

    /// The last item.
    pub fn last(&self) -> Option<&'m str> {
        self.items.last().map(|item| self.resolver.str(*item))
    }

    /// Every item in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'m str> + DoubleEndedIterator + use<'m> {
        let resolver = self.resolver;
        self.items.iter().map(move |item| resolver.str(*item))
    }

    /// Whether an item equals `text` exactly.
    pub fn contains(&self, text: &str) -> bool {
        self.iter().any(|item| item == text)
    }
}

impl fmt::Debug for Headers<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl fmt::Debug for Header<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("name", &self.raw_name())
            .field("value", &self.value())
            .field("offset_field", &self.offset_field())
            .field("offset_start", &self.offset_start())
            .field("offset_end", &self.offset_end())
            .finish()
    }
}

impl fmt::Debug for HeaderIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeaderIter")
            .field("remaining", &self.entries.len())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for NamedHeaders<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedHeaders")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for TextList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl PartialEq for TextList<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.items.len() == other.items.len() && self.iter().eq(other.iter())
    }
}

impl Eq for TextList<'_> {}
