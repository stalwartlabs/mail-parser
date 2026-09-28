/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{ContentType, Ctx, Headers, MessageRef, Resolver, TextList};
use crate::{
    Charset, Encoding, HeaderName,
    decoders::{self, html::strip_charset_meta},
    store::{KindTag, MessageId, NONE, PartEntry, PartId, flag, role},
};
use std::{borrow::Cow, fmt};

/// A MIME part: a header block and a body. Parts are numbered in document
/// order across nested messages ([`MessagePart::id`]), and every message has
/// a root part holding its own header fields.
///
/// Bodies are decoded on access: [`MessagePart::decoded`] for the transfer
/// encoding, [`MessagePart::text`] for text parts (transfer encoding and
/// charset), [`MessagePart::text_prefix`] and [`MessagePart::decoded_len`]
/// to avoid decoding the whole body.
#[derive(Clone, Copy)]
pub struct MessagePart<'m> {
    ctx: Ctx<'m>,
    id: PartId,
    entry: &'m PartEntry,
}

/// What a part contains.
#[derive(Debug, Clone, Copy)]
pub enum PartKind<'m> {
    /// A text part (any `text/*` except HTML, or a part without type). Also
    /// a `message/rfc822` part that is not transfer-encoded and whose nested
    /// message was empty or had no header block (flagged
    /// [`PartFlags::NO_BLANK_LINE`]), or was cut by the part limit
    /// ([`PartFlags::LIMIT_REACHED`]).
    Text,
    /// A `text/html` part.
    Html,
    /// A non-text part. Also a `message/rfc822` part that was not parsed:
    /// beyond the depth limit, or transfer-encoded and beyond the encoded
    /// nesting limit, with encoded nested parsing turned off, or holding a
    /// message that was empty or cut by the part limit.
    Binary,
    /// A non-text part marked to be rendered inline; see
    /// [`MessagePart::is_inline`].
    InlineBinary,
    /// A multipart container; see [`MessagePart::children`].
    Multipart,
    /// A nested `message/rfc822` or `message/global`.
    Message(MessageRef<'m>),
}

/// How the body classification used a part (issue #107). Every part has
/// exactly one role; the roles of a message's parts partition them.
///
/// The role is a summary of the list memberships
/// ([`MessagePart::in_text_body`], [`MessagePart::in_html_body`],
/// [`MessagePart::is_attachment`]). [`PartRole::CopiedBody`] takes priority
/// over [`PartRole::TextBody`] and [`PartRole::HtmlBody`]: a text or HTML
/// part copied into the other list is in both lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartRole {
    /// A multipart; its children have their own roles.
    Container,
    /// A text part of the body lists.
    TextBody,
    /// An HTML part of the body lists.
    HtmlBody,
    /// A body part of a `multipart/alternative` that had no version of the
    /// other type, copied into the other body list as its stand-in; it is in
    /// both body lists.
    CopiedBody,
    /// An image, audio or video part displayed in the body lists (it may
    /// also be an attachment; see [`MessageRef::attachments`]).
    MediaBody,
    /// A part offered as an attachment and in no body list.
    Attachment,
    /// A part in no list.
    Dropped,
}

macro_rules! flag_set {
    ($set:ident) => {
        impl $set {
            /// The flags as a bit mask.
            pub fn bits(&self) -> u8 {
                self.0
            }

            /// Whether no flag is set.
            pub fn is_empty(&self) -> bool {
                self.0 == 0
            }

            /// Whether every flag of `other` is set.
            pub fn contains(&self, other: $set) -> bool {
                self.0 & other.0 == other.0
            }

            /// Whether at least one flag of `other` is set.
            pub fn intersects(&self, other: $set) -> bool {
                self.0 & other.0 != 0
            }

            /// Names of the flags that are set, in bit order.
            pub fn names(&self) -> impl Iterator<Item = &'static str> + use<> {
                let flags = *self;
                <$set>::NAMES
                    .into_iter()
                    .filter(move |(value, _)| flags.contains(*value))
                    .map(|(_, name)| name)
            }
        }

        impl std::ops::BitOr for $set {
            type Output = $set;

            fn bitor(self, other: $set) -> $set {
                $set(self.0 | other.0)
            }
        }

        impl std::ops::BitOrAssign for $set {
            fn bitor_assign(&mut self, other: $set) {
                self.0 |= other.0;
            }
        }
    };
}

/// What happened while a part was parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PartFlags(u8);

flag_set!(PartFlags);

/// Problems found while decoding a body, returned by
/// [`MessagePart::decoded_checked`] and [`MessagePart::text_checked`]. A
/// non-empty set is what RFC 8621 calls `isEncodingProblem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DecodeProblems(u8);

flag_set!(DecodeProblems);

impl DecodeProblems {
    /// The transfer encoding had malformed sections: bytes outside the
    /// base64 alphabet were skipped, or malformed quoted-printable escapes
    /// were kept as written.
    pub const MALFORMED_TRANSFER_ENCODING: DecodeProblems = DecodeProblems(1);
    /// The Content-Transfer-Encoding is none of `7bit`, `8bit`, `binary`,
    /// `base64` and `quoted-printable`; the body was left as written.
    pub const UNKNOWN_TRANSFER_ENCODING: DecodeProblems = DecodeProblems(1 << 1);
    /// The charset is unknown, or not supported by this build (the
    /// multi-byte charsets need the `full_encoding` feature); the text was
    /// decoded as UTF-8.
    pub const UNKNOWN_CHARSET: DecodeProblems = DecodeProblems(1 << 2);
    /// Byte sequences that are not valid in the charset were replaced by
    /// U+FFFD. For UTF-7 also: 8-bit bytes were read as Latin-1, or an
    /// unterminated shift sequence at the end was dropped.
    pub const MALFORMED_CHARSET: DecodeProblems = DecodeProblems(1 << 3);

    pub(crate) const NAMES: [(DecodeProblems, &'static str); 4] = [
        (
            DecodeProblems::MALFORMED_TRANSFER_ENCODING,
            "malformed_transfer_encoding",
        ),
        (
            DecodeProblems::UNKNOWN_TRANSFER_ENCODING,
            "unknown_transfer_encoding",
        ),
        (DecodeProblems::UNKNOWN_CHARSET, "unknown_charset"),
        (DecodeProblems::MALFORMED_CHARSET, "malformed_charset"),
    ];
}

impl PartFlags {
    /// No delimiter of the enclosing multipart ended this part; it runs to
    /// the parent's next delimiter or to the end of the input. On a
    /// multipart: its first delimiter was its close delimiter, so it has no
    /// children and its content is the preamble in
    /// [`MessagePart::raw_body`].
    pub const MISSING_DELIMITER: PartFlags = PartFlags(flag::MISSING_DELIMITER as u8);
    /// The part ended at a delimiter found in the middle of a line. On a
    /// multipart: its first delimiter was found in the middle of a line,
    /// because no delimiter at the start of a line opened it (only its close
    /// delimiter, or a delimiter of an enclosing multipart, was found).
    pub const FALLBACK_DELIMITER: PartFlags = PartFlags(flag::FALLBACK_DELIMITER as u8);
    /// The header block did not end with a blank line: it runs to the end of
    /// the input, or a delimiter cut it, and the body is empty. On a
    /// `message/rfc822` part that is not [`PartKind::Message`]: the nested
    /// message was empty or had no header block, so the part keeps its body
    /// as text ([`PartKind::Text`], or [`PartKind::Binary`] when it was
    /// transfer-encoded).
    pub const NO_BLANK_LINE: PartFlags = PartFlags(flag::NO_BLANK_LINE as u8);
    /// A multipart without its close delimiter. Not set on the multiparts
    /// left open when the part limit stopped the parse.
    pub const UNTERMINATED: PartFlags = PartFlags(flag::UNTERMINATED as u8);
    /// A transfer-encoded nested message beyond the encoded nesting limit.
    pub const NESTING_LIMIT: PartFlags = PartFlags(flag::NESTING_LIMIT as u8);
    /// A limit was reached here. Depth limit: this multipart or
    /// `message/rfc822` part was not opened and is kept as a leaf. Part
    /// limit: the parse stopped inside this part, which holds the content
    /// that was not parsed. That is the last part when it is a leaf child of
    /// the innermost open multipart (it then runs to the end of its source),
    /// else that multipart, or the `message/rfc822` part whose nested
    /// message had no part yet; the containers still open all run to the
    /// end of their source.
    pub const LIMIT_REACHED: PartFlags = PartFlags(flag::LIMIT_REACHED as u8);

    pub(crate) const NAMES: [(PartFlags, &'static str); 6] = [
        (PartFlags::MISSING_DELIMITER, "missing_delimiter"),
        (PartFlags::FALLBACK_DELIMITER, "fallback_delimiter"),
        (PartFlags::NO_BLANK_LINE, "no_blank_line"),
        (PartFlags::UNTERMINATED, "unterminated"),
        (PartFlags::NESTING_LIMIT, "nesting_limit"),
        (PartFlags::LIMIT_REACHED, "limit_reached"),
    ];
}

impl<'m> MessagePart<'m> {
    pub(crate) fn new(ctx: Ctx<'m>, id: PartId, entry: &'m PartEntry) -> Self {
        MessagePart { ctx, id, entry }
    }

    #[inline]
    fn resolver(&self) -> Resolver<'m> {
        self.ctx.resolver(self.entry.message)
    }

    fn source(&self) -> &'m [u8] {
        self.ctx.source_of(self.entry.message)
    }

    fn slice(&self, start: u32, end: u32) -> &'m [u8] {
        self.source()
            .get(start as usize..end as usize)
            .unwrap_or_default()
    }

    /// The id of this part in [`crate::Message::parts`]: parts of every
    /// message are numbered in document order.
    pub fn id(&self) -> PartId {
        self.id
    }

    pub(crate) fn message_id(&self) -> MessageId {
        self.entry.message
    }

    /// The message this part belongs to.
    pub fn message(&self) -> MessageRef<'m> {
        self.ctx.message(self.entry.message)
    }

    /// The enclosing multipart; `None` for the root part of a message.
    pub fn parent(&self) -> Option<MessagePart<'m>> {
        (self.entry.parent != NONE)
            .then(|| self.ctx.part(self.entry.parent))
            .flatten()
    }

    /// What the part contains, from its Content-Type (and
    /// Content-Disposition for [`PartKind::InlineBinary`]).
    pub fn kind(&self) -> PartKind<'m> {
        match self.entry.kind {
            KindTag::Text => PartKind::Text,
            KindTag::Html => PartKind::Html,
            KindTag::Binary => PartKind::Binary,
            KindTag::InlineBinary => PartKind::InlineBinary,
            KindTag::Multipart => PartKind::Multipart,
            KindTag::Message => PartKind::Message(self.ctx.message(self.entry.children.start)),
        }
    }

    /// Whether the part is [`PartKind::Text`] or [`PartKind::Html`].
    pub fn is_text(&self) -> bool {
        matches!(self.entry.kind, KindTag::Text | KindTag::Html)
    }

    /// Whether the part is [`PartKind::Multipart`].
    pub fn is_multipart(&self) -> bool {
        self.entry.kind == KindTag::Multipart
    }

    /// Whether the part is [`PartKind::Message`].
    pub fn is_message(&self) -> bool {
        self.entry.kind == KindTag::Message
    }

    /// Children of a multipart, in order.
    pub fn children(
        &self,
    ) -> impl ExactSizeIterator<Item = MessagePart<'m>> + DoubleEndedIterator + use<'m> {
        let ctx = self.ctx;
        let ids = if self.entry.kind == KindTag::Multipart {
            ctx.ids(self.entry.children)
        } else {
            &[]
        };
        ids.iter().map(move |id| ctx.part_or_empty(*id))
    }

    /// The nested message of a `message/rfc822` part.
    pub fn nested(&self) -> Option<MessageRef<'m>> {
        (self.entry.kind == KindTag::Message).then(|| self.ctx.message(self.entry.children.start))
    }

    /// The header fields of this part.
    #[inline]
    pub fn headers(&self) -> Headers<'m> {
        Headers::new(
            self.resolver(),
            self.ctx.data(),
            self.ctx
                .data()
                .headers
                .get(self.entry.headers.range())
                .unwrap_or_default(),
        )
    }

    fn content(&self, index: u32) -> Option<ContentType<'m>> {
        let data = self.ctx.data();
        data.content_types
            .get(index as usize)
            .map(|entry| ContentType::new(self.resolver(), data, entry))
    }

    /// The last Content-Type field, when it parsed.
    pub fn content_type(&self) -> Option<ContentType<'m>> {
        self.content(self.entry.content_type)
    }

    /// The last Content-Disposition field, when it parsed.
    pub fn content_disposition(&self) -> Option<ContentType<'m>> {
        self.content(self.entry.content_disposition)
    }

    /// The Content-Disposition `filename`, else the Content-Type `name`.
    pub fn attachment_name(&self) -> Option<&'m str> {
        self.content_disposition()
            .and_then(|disposition| disposition.attribute("filename"))
            .or_else(|| self.content_type()?.attribute("name"))
    }

    /// Whether the Content-Type is `type_/subtype` (ASCII case-insensitive).
    pub fn is_content_type(&self, type_: &str, subtype: &str) -> bool {
        self.content_type().is_some_and(|content_type| {
            content_type.ctype().eq_ignore_ascii_case(type_)
                && content_type
                    .subtype()
                    .is_some_and(|value| value.eq_ignore_ascii_case(subtype))
        })
    }

    /// The Content-ID field, without angle brackets.
    pub fn content_id(&self) -> Option<&'m str> {
        self.headers().value(HeaderName::ContentId)?.as_text()
    }

    /// The Content-Description field, decoded.
    pub fn content_description(&self) -> Option<&'m str> {
        self.headers()
            .value(HeaderName::ContentDescription)?
            .as_text()
    }

    /// The Content-Location field.
    pub fn content_location(&self) -> Option<&'m str> {
        self.headers().value(HeaderName::ContentLocation)?.as_text()
    }

    /// The languages of the Content-Language field.
    pub fn content_language(&self) -> Option<TextList<'m>> {
        self.headers()
            .value(HeaderName::ContentLanguage)?
            .as_text_list()
    }

    /// The Content-Transfer-Encoding field as text: unfolded, trimmed, with
    /// encoded words decoded (the [`crate::HeaderForm::Text`] form), in the
    /// case it was written in. [`MessagePart::encoding`] gives the transfer
    /// encoding the body is decoded with.
    pub fn content_transfer_encoding(&self) -> Option<&'m str> {
        self.headers()
            .value(HeaderName::ContentTransferEncoding)?
            .as_text()
    }

    /// The declared transfer encoding: base64, quoted-printable, or none
    /// (`7bit`, `8bit`, `binary`, no field, or an unknown value).
    pub fn encoding(&self) -> Encoding {
        self.entry.encoding
    }

    /// What happened while the part was parsed.
    pub fn flags(&self) -> PartFlags {
        PartFlags((self.entry.flags & flag::MASK) as u8)
    }

    /// Whether the structure of this part was damaged: any of its
    /// [`PartFlags`] is set (this replaces 0.11's `is_encoding_problem`).
    /// Problems found while decoding the body are reported by
    /// [`MessagePart::decoded_checked`] and [`MessagePart::text_checked`].
    pub fn has_problems(&self) -> bool {
        self.entry.flags & flag::MASK != 0
    }

    /// Whether the part is in the text body of its message
    /// ([`MessageRef::text_body`]).
    pub fn in_text_body(&self) -> bool {
        self.entry.has_role(role::IN_TEXT_BODY)
    }

    /// Whether the part is in the HTML body of its message
    /// ([`MessageRef::html_body`]).
    pub fn in_html_body(&self) -> bool {
        self.entry.has_role(role::IN_HTML_BODY)
    }

    /// Whether the part is in the attachments of its message
    /// ([`MessageRef::attachments`]).
    pub fn is_attachment(&self) -> bool {
        self.entry.has_role(role::ATTACHMENT)
    }

    /// Whether the part is marked to be rendered inside the message rather
    /// than offered as a download (issue #70): `Content-Disposition: inline`,
    /// or no disposition on a child of `multipart/related` other than the
    /// first (the target of a `cid:` reference). `Content-Disposition:
    /// attachment` is never inline.
    ///
    /// This answers the disposition question only. It does not say whether
    /// the part is shown in a body list or offered as an attachment; see
    /// [`MessagePart::role`] and the list membership accessors for that.
    pub fn is_inline(&self) -> bool {
        self.entry.has_role(role::INLINE)
    }

    /// The classifier's decision for this part. A part copied into the
    /// other body list of a `multipart/alternative` is reported as
    /// [`PartRole::CopiedBody`], although it is also in its own list.
    pub fn role(&self) -> PartRole {
        let flags = self.entry.flags;
        match self.entry.kind {
            KindTag::Multipart => PartRole::Container,
            _ if flags & role::COPIED != 0 => PartRole::CopiedBody,
            kind if flags & (role::IN_TEXT_BODY | role::IN_HTML_BODY) != 0 => match kind {
                KindTag::Html => PartRole::HtmlBody,
                KindTag::Text => PartRole::TextBody,
                _ => PartRole::MediaBody,
            },
            _ if flags & role::ATTACHMENT != 0 => PartRole::Attachment,
            _ => PartRole::Dropped,
        }
    }

    /// The `boundary` parameter of a multipart part.
    pub fn boundary(&self) -> Option<&'m str> {
        if self.entry.kind == KindTag::Multipart {
            self.content_type()?.attribute("boundary")
        } else {
            None
        }
    }

    /// The boundary of the enclosing multipart, whose delimiter starts this
    /// part.
    pub fn delimiter(&self) -> Option<&'m str> {
        self.parent()?.boundary()
    }

    /// Headers and body, excluding the line break before the next
    /// delimiter.
    pub fn raw(&self) -> &'m [u8] {
        self.slice(self.entry.offset_header, self.entry.offset_end)
    }

    /// The header block, from the first header to the start of the body
    /// (the blank line included). The part ends, like any part, before the
    /// line break that precedes its delimiter; when that delimiter cuts the
    /// header block ([`PartFlags::NO_BLANK_LINE`]), or ends the part and its
    /// container right after the blank line, the header block stops there
    /// too and excludes that line break, which the last header field
    /// ([`crate::Header::offset_end`]) or the blank line still covers.
    pub fn raw_headers(&self) -> &'m [u8] {
        self.slice(self.entry.offset_header, self.entry.offset_body)
    }

    /// The body as written (still transfer-encoded), excluding the line
    /// break before the next delimiter.
    pub fn raw_body(&self) -> &'m [u8] {
        self.slice(self.entry.offset_body, self.entry.offset_end)
    }

    /// Offset of the first header of this part in the source buffer of its
    /// message ([`MessageRef::source`]).
    pub fn offset_header(&self) -> u32 {
        self.entry.offset_header
    }

    /// Offset of the body of this part in the source buffer of its message.
    pub fn offset_body(&self) -> u32 {
        self.entry.offset_body
    }

    /// Offset of the end of this part in the source buffer of its message.
    pub fn offset_end(&self) -> u32 {
        self.entry.offset_end
    }

    /// The transfer-decoded body; borrowed when nothing needs decoding.
    /// Malformed input is decoded on a best-effort basis: bytes outside the
    /// base64 alphabet are skipped, malformed quoted-printable escapes are
    /// kept as written. [`MessagePart::decoded_checked`] also reports those
    /// problems.
    pub fn decoded(&self) -> Cow<'m, [u8]> {
        self.entry.encoding.decode(self.raw_body())
    }

    /// Appends the transfer-decoded body to `out`, for reusing a buffer in
    /// a loop.
    pub fn decode_into(&self, out: &mut Vec<u8>) {
        self.entry.encoding.decode_append(self.raw_body(), out)
    }

    /// Length of the transfer-decoded body. Base64 and quoted-printable
    /// bodies are measured without being decoded, except base64 with bytes
    /// outside the alphabet.
    pub fn decoded_len(&self) -> usize {
        self.entry.encoding.decoded_len(self.raw_body())
    }

    /// [`MessagePart::decoded`] together with the problems found while
    /// decoding: malformed base64 or quoted-printable, or an unknown
    /// Content-Transfer-Encoding. The bytes are the ones `decoded()`
    /// returns; `decoded()` stays the faster call when the problems are not
    /// needed.
    pub fn decoded_checked(&self) -> (Cow<'m, [u8]>, DecodeProblems) {
        let (decoded, malformed) = self.entry.encoding.decode_checked(self.raw_body());
        let mut problems = DecodeProblems::default();
        if malformed {
            problems |= DecodeProblems::MALFORMED_TRANSFER_ENCODING;
        }
        if !self.has_known_transfer_encoding() {
            problems |= DecodeProblems::UNKNOWN_TRANSFER_ENCODING;
        }
        (decoded, problems)
    }

    /// False when the part declares a Content-Transfer-Encoding other than
    /// `7bit`, `8bit`, `binary`, `base64` or `quoted-printable`, the
    /// condition [`DecodeProblems::UNKNOWN_TRANSFER_ENCODING`] reports,
    /// without decoding the body. True when the field is absent.
    pub fn has_known_transfer_encoding(&self) -> bool {
        self.headers()
            .get(HeaderName::ContentTransferEncoding)
            .is_none_or(|header| Encoding::parse(header.raw_value()).is_some())
    }

    fn charset(&self) -> Option<&'m str> {
        self.content_type()?.attribute("charset")
    }

    fn text_charset(&self) -> Charset {
        self.charset()
            .and_then(|label| Charset::from_label(label.as_bytes()))
            .unwrap_or_default()
    }

    /// The text of a text or HTML part: transfer-decoded, then converted
    /// from its charset (UTF-8 when the charset is missing or unknown; byte
    /// sequences that are not valid in the charset become U+FFFD). Borrowed
    /// when the body needs no conversion. `None` for other parts.
    /// [`MessagePart::text_checked`] also reports decoding problems.
    pub fn text(&self) -> Option<Cow<'m, str>> {
        self.is_text()
            .then(|| self.text_charset().decode_cow(self.decoded()))
    }

    /// [`MessagePart::text`] together with the problems found while decoding
    /// it: those of [`MessagePart::decoded_checked`], an unknown charset,
    /// and byte sequences that are not valid in the charset. The text is the
    /// one `text()` returns. `None` for parts that are not text.
    ///
    /// For JMAP, `isEncodingProblem` is `!problems.is_empty()`.
    pub fn text_checked(&self) -> Option<(Cow<'m, str>, DecodeProblems)> {
        if !self.is_text() {
            return None;
        }
        let (decoded, mut problems) = self.decoded_checked();
        let charset = match self.charset() {
            None => Charset::default(),
            Some(label) => Charset::from_label(label.as_bytes()).unwrap_or_else(|| {
                problems |= DecodeProblems::UNKNOWN_CHARSET;
                Charset::default()
            }),
        };
        if !charset.is_supported() {
            problems |= DecodeProblems::UNKNOWN_CHARSET;
        }
        let (text, malformed) = charset.decode_cow_checked(decoded);
        if malformed {
            problems |= DecodeProblems::MALFORMED_CHARSET;
        }
        Some((text, problems))
    }

    /// The first `max_chars` characters of [`MessagePart::text`], decoding
    /// only as much of the body as they need. `None` for other parts.
    pub fn text_prefix(&self, max_chars: usize) -> Option<Cow<'m, str>> {
        self.text_prefix_state(decoders::Limit::Chars(max_chars))
            .map(|prefix| prefix.text)
    }

    pub(crate) fn text_prefix_state(
        &self,
        limit: decoders::Limit,
    ) -> Option<decoders::TextPrefix<'m>> {
        self.is_text().then(|| {
            decoders::TextPrefix::decode(
                self.raw_body(),
                self.entry.encoding,
                self.text_charset(),
                limit,
            )
        })
    }

    /// Appends the text of a text or HTML part to `out`; returns `false`
    /// for other parts.
    pub fn text_into(&self, out: &mut String) -> bool {
        if !self.is_text() {
            return false;
        }
        self.text_charset().decode_append(&self.decoded(), out);
        true
    }

    /// The HTML of an HTML part with its `<meta>` charset declarations
    /// rewritten to UTF-8 (issue #109), so that a browser reading the
    /// decoded text does not apply the original charset a second time.
    /// `None` for other parts.
    pub fn html_utf8(&self) -> Option<Cow<'m, str>> {
        if self.entry.kind != KindTag::Html {
            return None;
        }
        Some(decoders::rewrite(self.text()?, strip_charset_meta))
    }
}

impl fmt::Debug for MessagePart<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessagePart")
            .field("id", &self.id)
            .field("kind", &self.entry.kind)
            .field("offset_header", &self.entry.offset_header)
            .field("offset_body", &self.entry.offset_body)
            .field("offset_end", &self.entry.offset_end)
            .finish()
    }
}
