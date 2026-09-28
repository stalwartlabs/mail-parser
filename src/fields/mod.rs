/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

pub(crate) mod address;
pub(crate) mod content_type;
pub(crate) mod date;
pub(crate) mod id;
pub(crate) mod list;
pub(crate) mod raw;
pub(crate) mod received;
pub(crate) mod thread;
pub(crate) mod unstructured;

#[cfg(test)]
pub(crate) mod tests;

use crate::{
    HeaderValue,
    scan::Kernel,
    store::{
        AddressEntry, ContentTypeEntry, GROUP_BIT, MessageData, ParamEntry, ReceivedEntry, Span,
        Str, Value,
    },
    view::Resolver,
};
use std::{fmt, ops::Range};

/// How a header value is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HeaderForm {
    /// The value as written, without leading and trailing whitespace.
    Raw,
    /// Unstructured text with RFC 2047 encoded words decoded.
    Text,
    /// An address list: mailboxes and groups in document order.
    Addresses,
    /// A list of message identifiers.
    MessageIds,
    /// A comma-separated list of phrases.
    CommaList,
    /// An RFC 5322 date.
    Date,
    /// A Content-Type or Content-Disposition value.
    ContentType,
    /// A Received trace field.
    Received,
    /// Not parsed; the header is kept with an empty value.
    Ignore,
}

impl HeaderForm {
    /// Parses a standalone header value: the bytes after the colon, up to
    /// the end of the field (a line break not followed by white space).
    ///
    /// ```
    /// use mail_parser::HeaderForm;
    ///
    /// let parsed = HeaderForm::Addresses.parse(b" Jane <jane@example.com>, bob@example.com\r\n");
    /// let addresses = parsed.value().as_address().expect("addresses");
    /// assert_eq!(addresses.mailboxes().count(), 2);
    ///
    /// let parsed = HeaderForm::Date.parse(b" Sat, 20 Nov 2021 14:22:01 -0800\r\n");
    /// let date = parsed.value().as_datetime().expect("date");
    /// assert_eq!(date.to_timestamp(), 1_637_446_921);
    /// ```
    pub fn parse(self, value: &[u8]) -> ParsedValue<'_> {
        ParsedValue::new(self, value, 0..value.len())
    }
}

/// A header value parsed on its own, outside a message, by
/// [`HeaderForm::parse`] or [`crate::Header::parse_as`]. It borrows the
/// input and owns a small store only when the value needs one (a raw,
/// single-line text or date value allocates nothing). Parsed values compare
/// by their value.
#[derive(Clone)]
pub struct ParsedValue<'x> {
    source: &'x [u8],
    data: Option<MessageData>,
    value: Value,
}

static EMPTY_DATA: MessageData = MessageData::EMPTY;

impl<'x> ParsedValue<'x> {
    pub(crate) fn new(form: HeaderForm, source: &'x [u8], range: Range<usize>) -> Self {
        let range = first_field(source, range);
        let value = match form {
            HeaderForm::Ignore => Value::Empty,
            HeaderForm::Date => date::parse_bytes(source.get(range).unwrap_or_default())
                .map_or(Value::Empty, Value::DateTime),
            HeaderForm::Raw => match raw::borrowed(source, range.clone()) {
                Some(value) => value,
                None => return Self::stored(source, |ctx| raw::parse_raw(ctx, range)),
            },
            HeaderForm::Text => match unstructured::standalone(source, range) {
                Ok(value) => value,
                Err(pending) => return Self::stored(source, |ctx| pending.parse(ctx)),
            },
            HeaderForm::Addresses
            | HeaderForm::MessageIds
            | HeaderForm::CommaList
            | HeaderForm::ContentType
            | HeaderForm::Received => {
                return Self::stored(source, |ctx| parse_value(form, ctx, range));
            }
        };
        ParsedValue {
            source,
            data: None,
            value,
        }
    }

    fn stored(source: &'x [u8], parse: impl FnOnce(&mut FieldCtx<'_>) -> Value) -> Self {
        let mut data = MessageData::default();
        let value = parse(&mut FieldCtx::new(source, &mut data));
        ParsedValue {
            source,
            data: Some(data),
            value,
        }
    }

    /// The parsed value.
    pub fn value(&self) -> HeaderValue<'_> {
        let data = self.data.as_ref().unwrap_or(&EMPTY_DATA);
        HeaderValue::new(Resolver::new(self.source, &data.strings), data, self.value)
    }
}

impl PartialEq for ParsedValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
}

impl Eq for ParsedValue<'_> {}

impl fmt::Debug for ParsedValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParsedValue")
            .field("value", &self.value())
            .finish()
    }
}

pub(crate) fn first_field(source: &[u8], range: Range<usize>) -> Range<usize> {
    let end = Kernel::MEMCHR
        .field_end(source, range.start)
        .map_or(range.end, |newline| range.end.min(newline + 1));
    range.start..end
}

const FIRST_ADDRESSES: usize = 16;
const FIRST_PARAMS: usize = 16;
const FIRST_CONTENT_TYPES: usize = 16;
const FIRST_RECEIVED: usize = 8;

fn push_sized<T>(items: &mut Vec<T>, item: T, first: usize) {
    if items.capacity() == 0 {
        items.reserve_exact(first);
    }
    items.push(item);
}

pub(crate) fn push_utf8_lossy(out: &mut String, bytes: &[u8]) {
    for chunk in bytes.utf8_chunks() {
        out.push_str(chunk.valid());
        if !chunk.invalid().is_empty() {
            out.push(char::REPLACEMENT_CHARACTER);
        }
    }
}

pub(crate) fn is_fws(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

pub(crate) fn trim_fws(src: &[u8], range: Range<usize>) -> Range<usize> {
    let bytes = src.get(range.clone()).unwrap_or_default();
    let start = bytes.iter().position(|&byte| !is_fws(byte));
    let end = bytes.iter().rposition(|&byte| !is_fws(byte));
    match (start, end) {
        (Some(start), Some(end)) => range.start + start..range.start + end + 1,
        _ => range.start..range.start,
    }
}

pub(crate) fn parse_value(form: HeaderForm, ctx: &mut FieldCtx<'_>, value: Range<usize>) -> Value {
    match form {
        HeaderForm::Raw => raw::parse_raw(ctx, value),
        HeaderForm::Text => unstructured::parse_unstructured(ctx, value),
        HeaderForm::Addresses => address::parse_address(ctx, value),
        HeaderForm::MessageIds => id::parse_id(ctx, value),
        HeaderForm::CommaList => list::parse_comma_list(ctx, value),
        HeaderForm::Date => date::parse_date(ctx, value),
        HeaderForm::ContentType => content_type::parse_content_type(ctx, value),
        HeaderForm::Received => received::parse_received(ctx, value),
        HeaderForm::Ignore => Value::Empty,
    }
}

pub(crate) struct FieldCtx<'a> {
    src: &'a [u8],
    data: &'a mut MessageData,
    kernel: Kernel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupMark(u32);

impl<'a> FieldCtx<'a> {
    pub(crate) fn new(src: &'a [u8], data: &'a mut MessageData) -> Self {
        FieldCtx::with_kernel(src, data, Kernel::best())
    }

    pub(crate) fn with_kernel(src: &'a [u8], data: &'a mut MessageData, kernel: Kernel) -> Self {
        FieldCtx { src, data, kernel }
    }

    pub(crate) fn src(&self) -> &'a [u8] {
        self.src
    }

    pub(crate) fn kernel(&self) -> Kernel {
        self.kernel
    }

    pub(crate) fn bytes(&self, range: Range<usize>) -> &'a [u8] {
        self.src.get(range).unwrap_or_default()
    }

    pub(crate) fn resolve(&self, text: Str) -> &str {
        text.resolve(self.src, &self.data.strings)
    }

    pub(crate) fn try_borrow(&self, range: Range<usize>) -> Option<Str> {
        Str::borrow(self.src, range)
    }

    pub(crate) fn borrow(&mut self, range: Range<usize>) -> Str {
        match Str::borrow(self.src, range.clone()) {
            Some(text) => text,
            None => self.push_lossy(self.bytes(range)),
        }
    }

    pub(crate) fn push_str(&mut self, text: &str) -> Str {
        Str::push(&mut self.data.strings, text)
    }

    pub(crate) fn push_lossy(&mut self, bytes: &[u8]) -> Str {
        Str::push_with(&mut self.data.strings, |pool| push_utf8_lossy(pool, bytes))
    }

    pub(crate) fn push_with(&mut self, write: impl FnOnce(&mut String)) -> Str {
        Str::push_with(&mut self.data.strings, write)
    }

    pub(crate) fn lowercase(&mut self, range: Range<usize>) -> Str {
        let upper = self.bytes(range.clone()).iter().any(u8::is_ascii_uppercase);
        self.lowercase_scanned(range, upper)
    }

    pub(crate) fn lowercase_scanned(&mut self, range: Range<usize>, upper: bool) -> Str {
        if !upper && let Some(text) = Str::borrow(self.src, range.clone()) {
            return text;
        }
        let bytes = self.bytes(range);
        Str::push_with(&mut self.data.strings, |pool| {
            let start = pool.len();
            push_utf8_lossy(pool, bytes);
            if let Some(pushed) = pool.get_mut(start..) {
                pushed.make_ascii_lowercase();
            }
        })
    }

    pub(crate) fn trim_fws(&self, range: Range<usize>) -> Range<usize> {
        trim_fws(self.src, range)
    }

    pub(crate) fn take_text_scratch(&mut self) -> String {
        let mut text = std::mem::take(&mut self.data.scratch.text);
        text.clear();
        text
    }

    pub(crate) fn put_text_scratch(&mut self, text: String) {
        self.data.scratch.text = text;
    }

    pub(crate) fn take_bytes_scratch(&mut self) -> Vec<u8> {
        let mut bytes = std::mem::take(&mut self.data.scratch.bytes);
        bytes.clear();
        bytes
    }

    pub(crate) fn put_bytes_scratch(&mut self, bytes: Vec<u8>) {
        self.data.scratch.bytes = bytes;
    }

    pub(crate) fn text_items_mark(&self) -> u32 {
        self.data.text_items.len() as u32
    }

    pub(crate) fn push_text_item(&mut self, item: Str) {
        self.data.push_text_item(item);
    }

    pub(crate) fn text_list(&self, mark: u32) -> Value {
        match (self.data.text_items.len() as u32).saturating_sub(mark) {
            0 => Value::Empty,
            len => Value::TextList(Span { start: mark, len }),
        }
    }

    pub(crate) fn address_mark(&self) -> u32 {
        self.data.address_items.len() as u32
    }

    pub(crate) fn push_mailbox(&mut self, name: Option<Str>, address: Option<Str>) {
        self.data.address_items.push(
            AddressEntry {
                name: Str::from_option(name),
                address: Str::from_option(address),
                group: 0,
            },
            FIRST_ADDRESSES,
        );
    }

    pub(crate) fn push_group(&mut self, name: Option<Str>) -> GroupMark {
        let mark = GroupMark(self.data.address_items.len() as u32);
        self.data.address_items.push(
            AddressEntry {
                name: Str::from_option(name),
                address: Str::NONE,
                group: GROUP_BIT,
            },
            FIRST_ADDRESSES,
        );
        mark
    }

    pub(crate) fn end_group(&mut self, group: GroupMark) {
        let members = (self.data.address_items.len() as u32).saturating_sub(group.0 + 1);
        if let Some(entry) = self.data.address_items.get_mut(group.0 as usize) {
            entry.group = GROUP_BIT | members;
        }
    }

    pub(crate) fn address_list(&self, mark: u32) -> Value {
        let len = (self.data.address_items.len() as u32).saturating_sub(mark);
        if len == 0 {
            Value::Empty
        } else {
            Value::Address(Span { start: mark, len })
        }
    }

    pub(crate) fn params_mark(&self) -> u32 {
        self.data.params.len() as u32
    }

    pub(crate) fn push_param(&mut self, name: Str, value: Str) {
        self.data
            .params
            .push(ParamEntry { name, value }, FIRST_PARAMS);
    }

    pub(crate) fn params_since(&self, mark: u32) -> &[ParamEntry] {
        self.data.params.get(mark as usize..).unwrap_or_default()
    }

    pub(crate) fn set_param_value(&mut self, index: u32, value: Str) {
        if let Some(param) = self.data.params.get_mut(index as usize) {
            param.value = value;
        }
    }

    pub(crate) fn rollback_params(&mut self, mark: u32) {
        self.data.params.truncate(mark as usize);
    }

    pub(crate) fn remove_param(&mut self, index: u32) {
        if let Some(param) = self.data.params.get_mut(index as usize) {
            param.name = Str::NONE;
        }
    }

    pub(crate) fn drop_removed_params(&mut self, mark: u32) {
        let params = &mut self.data.params;
        let mut kept = mark as usize;
        for index in mark as usize..params.len() {
            let Some(&param) = params.get(index) else {
                break;
            };
            if param.name.is_some() {
                if let Some(slot) = params.get_mut(kept) {
                    *slot = param;
                }
                kept += 1;
            }
        }
        params.truncate(kept);
    }

    pub(crate) fn content_type(&mut self, ctype: Str, subtype: Option<Str>, params: u32) -> Value {
        let index = self.data.content_types.len() as u32;
        let params = Span {
            start: params,
            len: (self.data.params.len() as u32).saturating_sub(params),
        };
        self.data.content_types.push(
            ContentTypeEntry {
                ctype,
                subtype: Str::from_option(subtype),
                params,
            },
            FIRST_CONTENT_TYPES,
        );
        Value::ContentType(index)
    }

    pub(crate) fn received(&mut self, entry: ReceivedEntry) -> Value {
        let index = self.data.received.len() as u32;
        push_sized(&mut self.data.received, entry, FIRST_RECEIVED);
        Value::Received(index)
    }
}
