/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod builder;
mod classify;
mod delimiter;
mod header_block;

#[cfg(test)]
mod tests;

pub(crate) use builder::Frame;

use crate::{
    HeaderForm, HeaderName, Message, MessageBuffers,
    header_name::{FIRST_VENDOR, HeaderId, KNOWN_NAMES, OTHER_ID},
    scan::Kernel,
    store::MessageData,
};
use builder::Builder;
use std::borrow::Cow;

const FIRST_VENDOR_ID: usize = FIRST_VENDOR as usize;
const CONTENT_TYPE: u16 = HeaderId::ContentType as u16;
const CONTENT_DISPOSITION: u16 = HeaderId::ContentDisposition as u16;
const CONTENT_TRANSFER_ENCODING: u16 = HeaderId::ContentTransferEncoding as u16;
const DEFAULT_MAX_DEPTH: u32 = 64;
const DEFAULT_MAX_PARTS: u32 = 10_000;
const DEFAULT_MAX_ENCODED_NESTING: u32 = 3;

const TEXT: [HeaderId; 5] = [
    HeaderId::Subject,
    HeaderId::Comments,
    HeaderId::ContentDescription,
    HeaderId::ContentLocation,
    HeaderId::ContentTransferEncoding,
];

const ADDRESSES: [HeaderId; 18] = [
    HeaderId::From,
    HeaderId::To,
    HeaderId::Cc,
    HeaderId::Bcc,
    HeaderId::ReplyTo,
    HeaderId::Sender,
    HeaderId::ResentTo,
    HeaderId::ResentFrom,
    HeaderId::ResentBcc,
    HeaderId::ResentCc,
    HeaderId::ResentSender,
    HeaderId::ListArchive,
    HeaderId::ListHelp,
    HeaderId::ListId,
    HeaderId::ListOwner,
    HeaderId::ListPost,
    HeaderId::ListSubscribe,
    HeaderId::ListUnsubscribe,
];

const DATES: [HeaderId; 2] = [HeaderId::Date, HeaderId::ResentDate];

const MESSAGE_IDS: [HeaderId; 6] = [
    HeaderId::MessageId,
    HeaderId::References,
    HeaderId::InReplyTo,
    HeaderId::ReturnPath,
    HeaderId::ContentId,
    HeaderId::ResentMessageId,
];

const COMMA_LISTS: [HeaderId; 2] = [HeaderId::Keywords, HeaderId::ContentLanguage];

const CONTENT_TYPES: [HeaderId; 2] = [HeaderId::ContentType, HeaderId::ContentDisposition];

const fn assign(
    mut forms: [HeaderForm; KNOWN_NAMES],
    ids: &[HeaderId],
    form: HeaderForm,
) -> [HeaderForm; KNOWN_NAMES] {
    let mut index = 0;
    while index < ids.len() {
        forms[ids[index] as usize] = form;
        index += 1;
    }
    forms
}

const DEFAULT_FORMS: [HeaderForm; KNOWN_NAMES] = {
    let forms = [HeaderForm::Raw; KNOWN_NAMES];
    let forms = assign(forms, &TEXT, HeaderForm::Text);
    let forms = assign(forms, &ADDRESSES, HeaderForm::Addresses);
    let forms = assign(forms, &DATES, HeaderForm::Date);
    let forms = assign(forms, &MESSAGE_IDS, HeaderForm::MessageIds);
    let forms = assign(forms, &COMMA_LISTS, HeaderForm::CommaList);
    let forms = assign(forms, &[HeaderId::Received], HeaderForm::Received);
    assign(forms, &CONTENT_TYPES, HeaderForm::ContentType)
};

/// RFC 5322 message parser and its configuration.
///
/// Build a parser once and reuse it; it is `Send + Sync`. The default
/// configuration parses the header fields that 0.11 parsed into structured
/// values the same way and keeps every other field raw.
///
/// ```
/// use mail_parser::{HeaderForm, HeaderName, MessageParser};
///
/// let parser = MessageParser::new()
///     .header(HeaderName::Received, HeaderForm::Raw)
///     .header("X-Sender", HeaderForm::Addresses)
///     .max_parts(1_000);
/// let message = parser
///     .parse(b"X-Sender: <sender@example.com>\r\n\r\nbody")
///     .expect("message");
/// let sender = message.headers().value("x-sender").and_then(|value| value.as_address());
/// let address = sender.and_then(|list| list.first()).and_then(|mailbox| mailbox.address());
/// assert_eq!(address, Some("sender@example.com"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageParser {
    forms: [HeaderForm; KNOWN_NAMES],
    configured_vendor: Vec<u16>,
    custom: Vec<(Box<str>, HeaderForm)>,
    unknown: HeaderForm,
    max_depth: u32,
    max_parts: u32,
    max_encoded_nesting: u32,
    kernel: Kernel,
}

impl Default for MessageParser {
    fn default() -> Self {
        MessageParser::new()
    }
}

impl MessageParser {
    /// A parser with the default header table: every header 0.11 parsed
    /// into a structured value is parsed the same way, every other header is
    /// kept raw. Limits: nesting depth 64, 10,000 parts, 3 levels of
    /// transfer-encoded nested messages.
    pub fn new() -> Self {
        MessageParser {
            forms: DEFAULT_FORMS,
            configured_vendor: Vec::new(),
            custom: Vec::new(),
            unknown: HeaderForm::Raw,
            max_depth: DEFAULT_MAX_DEPTH,
            max_parts: DEFAULT_MAX_PARTS,
            max_encoded_nesting: DEFAULT_MAX_ENCODED_NESTING,
            kernel: Kernel::best(),
        }
    }

    /// Parses the header `name` as `form`, in place of its default form.
    /// Content-Type, Content-Disposition and Content-Transfer-Encoding
    /// always keep their MIME form. Names compare ASCII case-insensitively,
    /// and a [`HeaderName::Other`] spelled like a known name configures that
    /// name; any other name is added to a short list checked for every
    /// unknown name.
    pub fn header(mut self, name: impl Into<HeaderName<'static>>, form: HeaderForm) -> Self {
        let name = name.into();
        match (name.resolved_id(), name) {
            (CONTENT_TYPE | CONTENT_DISPOSITION | CONTENT_TRANSFER_ENCODING, _) => {}
            (OTHER_ID, HeaderName::Other(name)) if name.is_empty() => {}
            (OTHER_ID, HeaderName::Other(name)) => {
                match self
                    .custom
                    .iter_mut()
                    .find(|(custom, _)| custom.eq_ignore_ascii_case(&name))
                {
                    Some((_, custom_form)) => *custom_form = form,
                    None => self.custom.push((name.into_owned().into_boxed_str(), form)),
                }
            }
            (id, _) => {
                if let Some(slot) = self.forms.get_mut(usize::from(id)) {
                    *slot = form;
                    if id >= FIRST_VENDOR && !self.configured_vendor.contains(&id) {
                        self.configured_vendor.push(id);
                    }
                }
            }
        }
        self
    }

    /// The form used for header names that are neither known nor
    /// registered with [`MessageParser::header`]; [`HeaderForm::Raw`] by
    /// default.
    ///
    /// It also applies to the vendor names that are known since 1.0 but were
    /// unknown to 0.11 (from [`HeaderName::XOriginalTo`] on: X-Mailer,
    /// User-Agent, X-Priority, the X-MS-Exchange, X-Google and X-GitHub
    /// fields, ...), except those registered with `header`, whatever the
    /// order of the calls.
    pub fn unknown_headers(mut self, form: HeaderForm) -> Self {
        self.unknown = form;
        let vendor_forms = self.forms.get_mut(FIRST_VENDOR_ID..).unwrap_or_default();
        for (slot, id) in vendor_forms.iter_mut().zip(FIRST_VENDOR..) {
            if !self.configured_vendor.contains(&id) {
                *slot = form;
            }
        }
        self
    }

    /// Maximum number of nested containers (multiparts and messages), the
    /// root message included: with the default of 64, a part can be nested
    /// in at most 63 containers below the root message.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = u32::try_from(depth).unwrap_or(u32::MAX).max(1);
        self
    }

    /// Maximum number of parts; the content past the limit stays unparsed
    /// in the part flagged [`PartFlags::LIMIT_REACHED`](crate::PartFlags::LIMIT_REACHED).
    pub fn max_parts(mut self, parts: usize) -> Self {
        self.max_parts = u32::try_from(parts).unwrap_or(u32::MAX).max(1);
        self
    }

    /// Maximum nesting of transfer-encoded `message/rfc822` parts that are
    /// decoded and parsed; 0 leaves them as binary parts.
    pub fn max_encoded_nesting(mut self, levels: usize) -> Self {
        self.max_encoded_nesting = u32::try_from(levels).unwrap_or(u32::MAX);
        self
    }

    #[cfg(test)]
    pub(crate) fn kernel(mut self, kernel: Kernel) -> Self {
        self.kernel = kernel;
        self
    }

    pub(crate) fn known_form(&self, id: u16) -> HeaderForm {
        self.forms.get(id as usize).copied().unwrap_or(self.unknown)
    }

    pub(crate) fn other_form(&self, name: &[u8]) -> HeaderForm {
        self.custom
            .iter()
            .find(|(custom, _)| custom.as_bytes().eq_ignore_ascii_case(name))
            .map_or(self.unknown, |(_, form)| *form)
    }

    /// Parses a message that borrows `raw`. Returns `None` when the input
    /// has no header and no blank line, or is 4 GiB or larger.
    pub fn parse<'x>(&self, raw: &'x (impl AsRef<[u8]> + ?Sized)) -> Option<Message<'x>> {
        let mut message = Message::new(Cow::Borrowed(raw.as_ref()), MessageData::default());
        self.fill(&mut message, false).then_some(message)
    }

    /// Parses a message and takes ownership of its bytes, giving a
    /// `Message<'static>` that can be moved to another task.
    pub fn parse_owned(&self, raw: Vec<u8>) -> Option<Message<'static>> {
        let mut message = Message::new(Cow::Owned(raw), MessageData::default());
        self.fill(&mut message, false).then_some(message)
    }

    /// Parses only the root header block, for callers that do not need the
    /// MIME structure (DKIM verification, header indexing).
    ///
    /// The message has a single part, the root part, which covers the whole
    /// body and keeps the kind its Content-Type gives (a multipart root has
    /// no children). No part is in a body list or in the attachments:
    /// [`crate::MessageRef::has_attachments`] is false, and a root that is
    /// not a multipart is reported by [`crate::MessageRef::other_parts`].
    pub fn parse_headers<'x>(&self, raw: &'x (impl AsRef<[u8]> + ?Sized)) -> Option<Message<'x>> {
        let mut message = Message::new(Cow::Borrowed(raw.as_ref()), MessageData::default());
        self.fill(&mut message, true).then_some(message)
    }

    /// Parses a message into the storage held by `buffers`, which a
    /// previous message gave back with [`Message::into_buffers`]; once the
    /// storage fits the messages, parsing allocates nothing. See
    /// [`MessageBuffers`] for a loop.
    ///
    /// The message takes the storage out of `buffers`, which stay usable:
    /// when the input does not parse, the storage is dropped and the next
    /// parse allocates.
    pub fn parse_with<'x>(
        &self,
        raw: &'x (impl AsRef<[u8]> + ?Sized),
        buffers: &mut MessageBuffers,
    ) -> Option<Message<'x>> {
        let mut message = Message::new(
            Cow::Borrowed(raw.as_ref()),
            std::mem::take(&mut buffers.data),
        );
        message.split_mut().1.clear();
        self.fill(&mut message, false).then_some(message)
    }

    fn fill(&self, message: &mut Message<'_>, headers_only: bool) -> bool {
        let (raw, data) = message.split_mut();
        u32::try_from(raw.len()).is_ok() && Builder::new(self, data, headers_only).run(raw)
    }

    pub(crate) fn scan_kernel(&self) -> Kernel {
        self.kernel
    }

    pub(crate) fn limits(&self) -> (u32, u32, u32) {
        (self.max_depth, self.max_parts, self.max_encoded_nesting)
    }
}
