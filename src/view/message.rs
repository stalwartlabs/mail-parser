/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{
    AddressList, ContentType, Ctx, Headers, MessagePart, PartKind, PartRole, Received, TextList,
};
use crate::{
    DateTime, MessageBuffers,
    decoders::{self, Limit, preview::preview_html_prefix},
    html_to_text, preview_html, preview_text,
    store::{MessageData, MessageEntry, MessageId, NONE, PartId, Span},
    text_to_html,
};
use memchr::{memchr, memchr_iter};
use std::{borrow::Cow, fmt};

const PREVIEW_SLACK: usize = 16;
const PREVIEW_CR_ALLOWANCE: usize = 16;
const PREVIEW_GROWTH: usize = 4;
const PREVIEW_HTML_RATIO: usize = 8;
const PREVIEW_HTML_MIN: usize = 4096;

/// A parsed message: the raw bytes and a store of every nested message, MIME
/// part and header field, read through small `Copy` views such as
/// [`MessageRef`], [`MessagePart`] and [`crate::Header`]. Bodies are decoded
/// on access.
///
/// A message is read-only, `Send + Sync`, and covariant in `'x`, the
/// lifetime of the raw bytes; [`crate::MessageParser::parse_owned`] and
/// [`Message::into_owned`] give a `Message<'static>`. Every accessor of the
/// root [`MessageRef`] is also available here: `message.subject()` is
/// `message.root().subject()`.
///
/// ```
/// use mail_parser::{MessageParser, PartKind};
///
/// let raw = concat!(
///     "From: Ann <ann@example.com>\r\n",
///     "Subject: Report\r\n",
///     "Content-Type: multipart/mixed; boundary=\"b\"\r\n",
///     "\r\n",
///     "--b\r\n",
///     "Content-Type: text/plain; charset=utf-8\r\n",
///     "\r\n",
///     "See the attachment.\r\n",
///     "--b\r\n",
///     "Content-Type: application/pdf; name=report.pdf\r\n",
///     "Content-Transfer-Encoding: base64\r\n",
///     "\r\n",
///     "JVBERi0xLjQK\r\n",
///     "--b--\r\n",
/// );
/// let message = MessageParser::new().parse(raw).expect("message");
/// assert_eq!(message.subject(), Some("Report"));
/// let sender = message.from().and_then(|from| from.first());
/// assert_eq!(sender.and_then(|mailbox| mailbox.name()), Some("Ann"));
/// assert_eq!(message.body_text(0).as_deref(), Some("See the attachment."));
/// let attachment = message.attachments().next().expect("attachment");
/// assert_eq!(attachment.attachment_name(), Some("report.pdf"));
/// assert!(matches!(attachment.kind(), PartKind::Binary));
/// assert_eq!(attachment.decoded().as_ref(), b"%PDF-1.4\n");
/// ```
#[derive(Clone)]
pub struct Message<'x> {
    raw: Cow<'x, [u8]>,
    data: MessageData,
}

/// A message inside a [`Message`]: the root message, or a message nested in
/// a `message/rfc822` (or `message/global`) part. Each message has its own
/// root part, header fields and body lists.
#[derive(Clone, Copy)]
pub struct MessageRef<'m> {
    ctx: Ctx<'m>,
    id: MessageId,
    entry: &'m MessageEntry,
}

/// The buffer the offsets of a message's parts and headers refer to.
///
/// A nested message that was not transfer-encoded is parsed in place, so its
/// offsets refer to the buffer of the message that contains it: the raw
/// input, or the decoded body of an encoded ancestor. A base64 or
/// quoted-printable encoded one is decoded during the parse and its offsets
/// refer to that decoded body, available from [`MessageRef::source_bytes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// The raw input, [`Message::raw`].
    Raw,
    /// The transfer-decoded body of this `message/rfc822` part: the part
    /// holding the message when the message was transfer-encoded, else the
    /// closest transfer-encoded `message/rfc822` part it is nested in.
    Decoded(PartId),
}

macro_rules! header_getters {
    ($lt:lifetime, $target:ident, $($captures:tt)*) => {
        /// The Subject field, RFC 2047 encoded words decoded.
        pub fn subject(&self) -> Option<&$lt str> {
            self.$target().subject()
        }

        /// The Comments field, decoded.
        pub fn comments(&self) -> Option<&$lt str> {
            self.$target().comments()
        }

        /// The MIME-Version field, as written.
        pub fn mime_version(&self) -> Option<&$lt str> {
            self.$target().mime_version()
        }

        /// The subject with reply and forward markers removed; see
        /// [`crate::thread_name`].
        pub fn thread_name(&self) -> Option<&$lt str> {
            self.$target().thread_name()
        }

        /// The From field.
        pub fn from(&self) -> Option<AddressList<$lt>> {
            self.$target().from()
        }

        /// The To field.
        pub fn to(&self) -> Option<AddressList<$lt>> {
            self.$target().to()
        }

        /// The Cc field.
        pub fn cc(&self) -> Option<AddressList<$lt>> {
            self.$target().cc()
        }

        /// The Bcc field.
        pub fn bcc(&self) -> Option<AddressList<$lt>> {
            self.$target().bcc()
        }

        /// The Reply-To field.
        pub fn reply_to(&self) -> Option<AddressList<$lt>> {
            self.$target().reply_to()
        }

        /// The Sender field.
        pub fn sender(&self) -> Option<AddressList<$lt>> {
            self.$target().sender()
        }

        /// Every To field, in document order.
        pub fn all_to(&self) -> impl Iterator<Item = AddressList<$lt>> $($captures)* {
            self.$target().all_to()
        }

        /// Every Cc field, in document order.
        pub fn all_cc(&self) -> impl Iterator<Item = AddressList<$lt>> $($captures)* {
            self.$target().all_cc()
        }

        /// Every Bcc field, in document order.
        pub fn all_bcc(&self) -> impl Iterator<Item = AddressList<$lt>> $($captures)* {
            self.$target().all_bcc()
        }

        /// The Resent-To field.
        pub fn resent_to(&self) -> Option<AddressList<$lt>> {
            self.$target().resent_to()
        }

        /// The Resent-From field.
        pub fn resent_from(&self) -> Option<AddressList<$lt>> {
            self.$target().resent_from()
        }

        /// The Resent-Cc field.
        pub fn resent_cc(&self) -> Option<AddressList<$lt>> {
            self.$target().resent_cc()
        }

        /// The Resent-Bcc field.
        pub fn resent_bcc(&self) -> Option<AddressList<$lt>> {
            self.$target().resent_bcc()
        }

        /// The Resent-Sender field.
        pub fn resent_sender(&self) -> Option<AddressList<$lt>> {
            self.$target().resent_sender()
        }

        /// The List-Archive field; see [`Headers::list_archive`].
        pub fn list_archive(&self) -> Option<AddressList<$lt>> {
            self.$target().list_archive()
        }

        /// The List-Help field; see [`Headers::list_help`].
        pub fn list_help(&self) -> Option<AddressList<$lt>> {
            self.$target().list_help()
        }

        /// The List-ID field; see [`Headers::list_id`].
        pub fn list_id(&self) -> Option<AddressList<$lt>> {
            self.$target().list_id()
        }

        /// The List-Owner field; see [`Headers::list_owner`].
        pub fn list_owner(&self) -> Option<AddressList<$lt>> {
            self.$target().list_owner()
        }

        /// The List-Post field; see [`Headers::list_post`].
        pub fn list_post(&self) -> Option<AddressList<$lt>> {
            self.$target().list_post()
        }

        /// The List-Subscribe field; see [`Headers::list_subscribe`].
        pub fn list_subscribe(&self) -> Option<AddressList<$lt>> {
            self.$target().list_subscribe()
        }

        /// The List-Unsubscribe field; see [`Headers::list_unsubscribe`].
        pub fn list_unsubscribe(&self) -> Option<AddressList<$lt>> {
            self.$target().list_unsubscribe()
        }

        /// The Date field.
        pub fn date(&self) -> Option<DateTime> {
            self.$target().date()
        }

        /// The Resent-Date field.
        pub fn resent_date(&self) -> Option<DateTime> {
            self.$target().resent_date()
        }

        /// The last identifier of the Message-ID field, without angle
        /// brackets.
        pub fn message_id(&self) -> Option<&$lt str> {
            self.$target().message_id()
        }

        /// The identifiers of the In-Reply-To field.
        pub fn in_reply_to(&self) -> Option<TextList<$lt>> {
            self.$target().in_reply_to()
        }

        /// The identifiers of the References field.
        pub fn references(&self) -> Option<TextList<$lt>> {
            self.$target().references()
        }

        /// The identifiers of the Resent-Message-ID field.
        pub fn resent_message_id(&self) -> Option<TextList<$lt>> {
            self.$target().resent_message_id()
        }

        /// The Return-Path field; see [`Headers::return_path`].
        pub fn return_path(&self) -> Option<TextList<$lt>> {
            self.$target().return_path()
        }

        /// The phrases of the Keywords field.
        pub fn keywords(&self) -> Option<TextList<$lt>> {
            self.$target().keywords()
        }

        /// The last address of Return-Path, or else the first address of
        /// From.
        pub fn return_address(&self) -> Option<&$lt str> {
            self.$target().return_address()
        }

        /// The last Received field (the oldest hop).
        pub fn received(&self) -> Option<Received<$lt>> {
            self.$target().received()
        }

        /// Every Received field, in document order (newest hop first).
        pub fn all_received(&self) -> impl Iterator<Item = Received<$lt>> $($captures)* {
            self.$target().all_received()
        }
    };
}

macro_rules! message_getters {
    ($lt:lifetime) => {
        /// The root part of the root message; see [`MessageRef::root_part`].
        pub fn root_part(&self) -> MessagePart<$lt> {
            self.root().root_part()
        }

        /// The header fields of the root message.
        pub fn headers(&self) -> Headers<$lt> {
            self.root().headers()
        }

        /// The text body of the root message; see [`MessageRef::text_body`].
        pub fn text_body(&self) -> impl ExactSizeIterator<Item = MessagePart<$lt>> {
            self.root().text_body()
        }

        /// The HTML body of the root message; see [`MessageRef::html_body`].
        pub fn html_body(&self) -> impl ExactSizeIterator<Item = MessagePart<$lt>> {
            self.root().html_body()
        }

        /// The attachments of the root message; see
        /// [`MessageRef::attachments`].
        pub fn attachments(&self) -> impl ExactSizeIterator<Item = MessagePart<$lt>> {
            self.root().attachments()
        }

        /// The parts of the root message in no list; see
        /// [`MessageRef::other_parts`].
        pub fn other_parts(&self) -> impl Iterator<Item = MessagePart<$lt>> {
            self.root().other_parts()
        }

        /// Whether the root message has attachments; see
        /// [`MessageRef::has_attachments`].
        pub fn has_attachments(&self) -> bool {
            self.root().has_attachments()
        }

        /// The text body of the root message as text; see
        /// [`MessageRef::text_bodies`].
        pub fn text_bodies(&self) -> impl Iterator<Item = Cow<$lt, str>> {
            self.root().text_bodies()
        }

        /// The HTML body of the root message as HTML; see
        /// [`MessageRef::html_bodies`].
        pub fn html_bodies(&self) -> impl Iterator<Item = Cow<$lt, str>> {
            self.root().html_bodies()
        }

        /// The text of the `index`-th part of the root text body; see
        /// [`MessageRef::body_text`].
        pub fn body_text(&self, index: usize) -> Option<Cow<$lt, str>> {
            self.root().body_text(index)
        }

        /// The HTML of the `index`-th part of the root HTML body; see
        /// [`MessageRef::body_html`].
        pub fn body_html(&self, index: usize) -> Option<Cow<$lt, str>> {
            self.root().body_html(index)
        }

        /// A preview of the root message's body; see
        /// [`MessageRef::body_preview`].
        pub fn body_preview(&self, max_len: usize) -> Option<Cow<$lt, str>> {
            self.root().body_preview(max_len)
        }

        /// The Content-Type of the root part.
        pub fn content_type(&self) -> Option<ContentType<$lt>> {
            self.root().root_part().content_type()
        }

        /// The Content-Disposition of the root part.
        pub fn content_disposition(&self) -> Option<ContentType<$lt>> {
            self.root().root_part().content_disposition()
        }

        /// The attachment name of the root part; see
        /// [`MessagePart::attachment_name`].
        pub fn attachment_name(&self) -> Option<&$lt str> {
            self.root().root_part().attachment_name()
        }

        /// Whether the Content-Type of the root part is `type_/subtype`
        /// (ASCII case-insensitive).
        pub fn is_content_type(&self, type_: &str, subtype: &str) -> bool {
            self.root().root_part().is_content_type(type_, subtype)
        }

        header_getters!($lt, headers,);
    };
}

impl Default for Message<'_> {
    /// An empty message: no bytes, and a root part with no header and an
    /// empty body, in no body list.
    fn default() -> Self {
        Message {
            raw: Cow::Borrowed(&[]),
            data: MessageData::empty_message(),
        }
    }
}

impl<'x> Message<'x> {
    pub(crate) fn new(raw: Cow<'x, [u8]>, data: MessageData) -> Self {
        Message { raw, data }
    }

    pub(crate) fn split_mut(&mut self) -> (&[u8], &mut MessageData) {
        (&self.raw, &mut self.data)
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx::new(&self.raw, &self.data)
    }

    /// The raw input.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The top-level message, id 0.
    pub fn root(&self) -> MessageRef<'_> {
        self.ctx().message(0)
    }

    /// Every message, the root first, then nested messages in document
    /// order.
    pub fn messages(&self) -> impl ExactSizeIterator<Item = MessageRef<'_>> + DoubleEndedIterator {
        let ctx = self.ctx();
        (0..self.data.messages.len() as MessageId).map(move |id| ctx.message(id))
    }

    /// Every part of every message, in document order.
    pub fn parts(&self) -> impl ExactSizeIterator<Item = MessagePart<'_>> + DoubleEndedIterator {
        let ctx = self.ctx();
        (0..self.data.parts.len() as PartId).map(move |id| ctx.part_or_empty(id))
    }

    /// The part with this id; ids number every part of every message in
    /// document order.
    pub fn part(&self, id: PartId) -> Option<MessagePart<'_>> {
        self.ctx().part(id)
    }

    /// The multipart part whose `boundary` parameter is `boundary` (issue
    /// #127); a linear scan over the parts.
    pub fn part_by_boundary(&self, boundary: &str) -> Option<MessagePart<'_>> {
        self.parts().find(|part| part.boundary() == Some(boundary))
    }

    /// Copies borrowed input so the message owns its bytes. Only the raw
    /// bytes are copied; the parsed store is moved.
    pub fn into_owned(self) -> Message<'static> {
        Message {
            raw: Cow::Owned(self.raw.into_owned()),
            data: self.data,
        }
    }

    /// Drops the message and keeps its storage, cleared, for the next
    /// [`crate::MessageParser::parse_with`]; see [`MessageBuffers`].
    pub fn into_buffers(self) -> MessageBuffers {
        let mut data = self.data;
        data.clear();
        MessageBuffers { data }
    }

    message_getters!('_);
}

impl<'m> MessageRef<'m> {
    pub(crate) fn new(ctx: Ctx<'m>, id: MessageId, entry: &'m MessageEntry) -> Self {
        MessageRef { ctx, id, entry }
    }

    fn list(
        &self,
        skip: u32,
        len: u32,
    ) -> impl ExactSizeIterator<Item = MessagePart<'m>> + use<'m> {
        let ctx = self.ctx;
        ctx.ids(Span {
            start: self.entry.lists + skip,
            len,
        })
        .iter()
        .map(move |id| ctx.part_or_empty(*id))
    }

    /// The id of this message: 0 for the root, then nested messages in
    /// document order.
    pub fn id(&self) -> MessageId {
        self.id
    }

    /// The root part: the header block of this message and its body.
    pub fn root_part(&self) -> MessagePart<'m> {
        self.ctx.part_or_empty(self.entry.root)
    }

    /// The `message/rfc822` part holding this message; `None` for the root
    /// message.
    pub fn container(&self) -> Option<MessagePart<'m>> {
        (self.entry.container != NONE)
            .then(|| self.ctx.part(self.entry.container))
            .flatten()
    }

    /// The buffer this message's offsets refer to.
    pub fn source(&self) -> Source {
        self.ctx
            .data()
            .source_container(self.entry.source)
            .map_or(Source::Raw, Source::Decoded)
    }

    /// The bytes of the buffer this message's offsets refer to.
    pub fn source_bytes(&self) -> &'m [u8] {
        self.ctx.source_of(self.id)
    }

    /// The bytes of this message, from its first header to its end.
    pub fn raw(&self) -> &'m [u8] {
        self.root_part().raw()
    }

    /// The header fields of this message: those of its root part.
    pub fn headers(&self) -> Headers<'m> {
        self.root_part().headers()
    }

    /// The parts of this message (nested messages excluded), in document
    /// order.
    pub fn parts(&self) -> impl Iterator<Item = MessagePart<'m>> + use<'m> {
        let ctx = self.ctx;
        let id = self.id;
        let end = if self.entry.parts_end == NONE {
            ctx.data().parts.len() as PartId
        } else {
            self.entry.parts_end
        };
        let start = if self.entry.root == NONE {
            end
        } else {
            self.entry.root
        };
        (start..end)
            .filter_map(move |part| ctx.part(part))
            .filter(move |part| part.message_id() == id)
    }

    /// The parts that make up the text body (RFC 8621 `textBody`): text
    /// parts, HTML parts standing in for a missing text version, and inline
    /// media, in reading order. [`MessageRef::text_bodies`] gives them as
    /// text.
    pub fn text_body(&self) -> impl ExactSizeIterator<Item = MessagePart<'m>> + use<'m> {
        self.list(0, self.entry.text_len)
    }

    /// The parts that make up the HTML body (RFC 8621 `htmlBody`): HTML
    /// parts, text parts standing in for a missing HTML version, and inline
    /// media, in reading order. [`MessageRef::html_bodies`] gives them as
    /// HTML.
    pub fn html_body(&self) -> impl ExactSizeIterator<Item = MessagePart<'m>> + use<'m> {
        self.list(self.entry.text_len, self.entry.html_len)
    }

    /// The attachments (RFC 8621 `attachments`): the leaf parts, nested
    /// messages included, that are not shown as part of the body, plus inline
    /// media shown in only one of the two bodies.
    pub fn attachments(&self) -> impl ExactSizeIterator<Item = MessagePart<'m>> + use<'m> {
        self.list(
            self.entry.text_len + self.entry.html_len,
            self.entry.attachments_len,
        )
    }

    /// Whether a client should offer this message as having attachments:
    /// RFC 8621 `hasAttachment`, true when [`MessageRef::attachments`] holds
    /// a part without `Content-Disposition: inline`. Images referenced by
    /// `cid:` count unless they are marked inline; the RFC's optional
    /// exception for embedded images is not applied.
    pub fn has_attachments(&self) -> bool {
        self.attachments().any(|part| {
            !part
                .content_disposition()
                .is_some_and(|disposition| disposition.is_inline())
        })
    }

    /// The parts of this message that are in no list (issue #107):
    /// containers excluded, nested messages excluded. For a message from
    /// [`crate::MessageParser::parse_headers`] this is the root part.
    pub fn other_parts(&self) -> impl Iterator<Item = MessagePart<'m>> + use<'m> {
        self.parts().filter(|part| part.role() == PartRole::Dropped)
    }

    fn as_text(part: MessagePart<'m>) -> Option<Cow<'m, str>> {
        match part.kind() {
            PartKind::Text => part.text(),
            PartKind::Html => part.text().map(|html| Cow::Owned(html_to_text(&html))),
            _ => None,
        }
    }

    fn as_html(part: MessagePart<'m>) -> Option<Cow<'m, str>> {
        match part.kind() {
            PartKind::Html => part.text(),
            PartKind::Text => part.text().map(|text| Cow::Owned(text_to_html(&text))),
            _ => None,
        }
    }

    /// The text body as text: HTML parts converted, non-text parts skipped.
    pub fn text_bodies(&self) -> impl Iterator<Item = Cow<'m, str>> + use<'m> {
        self.text_body().filter_map(Self::as_text)
    }

    /// The HTML body as HTML: text parts converted, non-text parts skipped.
    pub fn html_bodies(&self) -> impl Iterator<Item = Cow<'m, str>> + use<'m> {
        self.html_body().filter_map(Self::as_html)
    }

    /// The text of the `index`-th part of the text body.
    pub fn body_text(&self, index: usize) -> Option<Cow<'m, str>> {
        self.text_body().nth(index).and_then(Self::as_text)
    }

    /// The HTML of the `index`-th part of the HTML body.
    pub fn body_html(&self, index: usize) -> Option<Cow<'m, str>> {
        self.html_body().nth(index).and_then(Self::as_html)
    }

    /// A preview of at most `max_len` bytes of the first text body, else of
    /// the first HTML body, without carriage returns; see [`preview_text`].
    pub fn body_preview(&self, max_len: usize) -> Option<Cow<'m, str>> {
        if self.entry.text_len > 0 {
            let part = self.text_body().next()?;
            match part.kind() {
                PartKind::Text => Self::preview_text_part(part, max_len),
                PartKind::Html => Self::preview_html_part(part, max_len),
                _ => None,
            }
        } else if self.entry.html_len > 0 {
            let part = self.html_body().next()?;
            match part.kind() {
                PartKind::Html => Self::preview_html_part(part, max_len),
                PartKind::Text => Some(Cow::Owned(preview_html(
                    &text_to_html(&part.text()?),
                    max_len,
                ))),
                _ => None,
            }
        } else {
            None
        }
    }

    fn preview_text_part(part: MessagePart<'m>, max_len: usize) -> Option<Cow<'m, str>> {
        let mut bytes = max_len
            .saturating_add(max_len / PREVIEW_CR_ALLOWANCE)
            .saturating_add(PREVIEW_SLACK);
        loop {
            let prefix = part.text_prefix_state(Limit::Bytes(bytes))?;
            if prefix.complete || exceeds_without_cr(prefix.text.as_bytes(), max_len) {
                return Some(decoders::rewrite(prefix.text, |text| {
                    preview_text(text, max_len)
                }));
            }
            bytes = bytes.saturating_mul(PREVIEW_GROWTH);
        }
    }

    fn preview_html_part(part: MessagePart<'m>, max_len: usize) -> Option<Cow<'m, str>> {
        let mut bytes = max_len
            .saturating_mul(PREVIEW_HTML_RATIO)
            .max(PREVIEW_HTML_MIN);
        loop {
            let prefix = part.text_prefix_state(Limit::Bytes(bytes))?;
            if let Some(preview) = preview_html_prefix(&prefix.text, max_len, prefix.complete) {
                return Some(Cow::Owned(preview));
            }
            bytes = bytes.saturating_mul(PREVIEW_GROWTH);
        }
    }

    /// The Content-Type of the root part.
    pub fn content_type(&self) -> Option<ContentType<'m>> {
        self.root_part().content_type()
    }

    /// The Content-Disposition of the root part.
    pub fn content_disposition(&self) -> Option<ContentType<'m>> {
        self.root_part().content_disposition()
    }

    /// The attachment name of the root part; see
    /// [`MessagePart::attachment_name`].
    pub fn attachment_name(&self) -> Option<&'m str> {
        self.root_part().attachment_name()
    }

    /// Whether the Content-Type of the root part is `type_/subtype` (ASCII
    /// case-insensitive).
    pub fn is_content_type(&self, type_: &str, subtype: &str) -> bool {
        self.root_part().is_content_type(type_, subtype)
    }

    header_getters!('m, headers, + use<'m>);
}

fn exceeds_without_cr(text: &[u8], max_len: usize) -> bool {
    let needed = max_len.saturating_add(1);
    match text.get(..needed) {
        Some(window) if memchr(b'\r', window).is_none() => true,
        _ => text.len() - memchr_iter(b'\r', text).count() >= needed,
    }
}

impl fmt::Debug for Message<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("len", &self.raw.len())
            .field("messages", &self.data.messages.len())
            .field("parts", &self.data.parts.len())
            .field("headers", &self.data.headers.len())
            .finish()
    }
}

impl fmt::Debug for MessageRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageRef")
            .field("id", &self.id)
            .field("root", &self.entry.root)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::exceeds_without_cr;
    use crate::scan::tests::Rng;
    use memchr::memchr_iter;

    #[test]
    fn carriage_returns_do_not_count() {
        let mut rng = Rng(0x5eed_cafe_f00d_0001);
        for _ in 0..20_000 {
            let len = rng.below(40);
            let text: Vec<u8> = (0..len)
                .map(|_| if rng.below(3) == 0 { b'\r' } else { b'a' })
                .collect();
            let max_len = rng.below(40);
            let expected = text.len() - memchr_iter(b'\r', &text).count() > max_len;
            assert_eq!(
                exceeds_without_cr(&text, max_len),
                expected,
                "{text:?} {max_len}"
            );
        }
    }
}
