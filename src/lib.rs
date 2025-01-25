/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */
#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms)]
#[forbid(unsafe_code)]
pub mod core;
pub mod decoders;
pub mod mailbox;
pub mod parsers;

use std::{borrow::Cow, collections::HashMap, hash::Hash, net::IpAddr};

use parsers::MessageStream;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// RFC5322/RFC822 message parser.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MessageParser {
    pub(crate) header_map: HashMap<HeaderName<'static>, HdrParseFnc>,
    pub(crate) def_hdr_parse_fnc: HdrParseFnc,
}

pub(crate) type HdrParseFnc = for<'x> fn(&mut MessageStream<'x>) -> crate::HeaderValue<'x>;

/// An RFC5322/RFC822 message.
#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Message<'x> {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub html_body: Vec<MessagePartId>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub text_body: Vec<MessagePartId>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub attachments: Vec<MessagePartId>,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub parts: Vec<MessagePart<'x>>,

    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub raw_message: Cow<'x, [u8]>,
}

/// MIME Message Part
#[derive(Debug, PartialEq, Default, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct MessagePart<'x> {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub headers: Vec<Header<'x>>,
    pub is_encoding_problem: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub body: PartType<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub encoding: Encoding,
    pub offset_header: usize,
    pub offset_body: usize,
    pub offset_end: usize,
}

/// MIME Part encoding type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum Encoding {
    #[default]
    None = 0,
    QuotedPrintable = 1,
    Base64 = 2,
}

impl From<u8> for Encoding {
    fn from(v: u8) -> Self {
        match v {
            1 => Encoding::QuotedPrintable,
            2 => Encoding::Base64,
            _ => Encoding::None,
        }
    }
}

/// Unique ID representing a MIME part within a message.
pub type MessagePartId = usize;

/// A text, binary or nested e-mail MIME message part.
///
/// - Text: Any text/* part
/// - Binary: Any other part type that is not text.
/// - Message: Nested RFC5322 message.
/// - MultiPart: Multipart part.
///
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum PartType<'x> {
    /// Any text/* part
    Text(Cow<'x, str>),

    /// A text/html part
    Html(Cow<'x, str>),

    /// Any other part type that is not text.
    Binary(Cow<'x, [u8]>),

    /// Any inline binary data that.
    InlineBinary(Cow<'x, [u8]>),

    /// Nested RFC5322 message.
    Message(Message<'x>),

    /// Multipart part
    Multipart(Vec<MessagePartId>),
}

impl Default for PartType<'_> {
    fn default() -> Self {
        PartType::Multipart(Vec::with_capacity(0))
    }
}

/// An RFC5322 or RFC2369 internet address.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Addr<'x> {
    /// The address name including comments
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name: Option<Cow<'x, str>>,

    /// An e-mail address (RFC5322/RFC2369) or URL (RFC2369)
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub address: Option<Cow<'x, str>>,
}

/// An RFC5322 address group.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Group<'x> {
    /// Group name
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name: Option<Cow<'x, str>>,

    /// Addresses member of the group
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub addresses: Vec<Addr<'x>>,
}

/// A message header.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Header<'x> {
    pub name: HeaderName<'x>,
    pub value: HeaderValue<'x>,
    pub offset_field: usize,
    pub offset_start: usize,
    pub offset_end: usize,
}

/// A header field
#[derive(Debug, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum HeaderName<'x> {
    ArcAuthenticationResults,
    ArcMessageSignature,
    ArcSeal,
    Subject,
    From,
    To,
    Cc,
    Date,
    Bcc,
    ReplyTo,
    Sender,
    Comments,
    InReplyTo,
    Keywords,
    Received,
    MessageId,
    References,
    ReturnPath,
    MimeVersion,
    ContentDescription,
    ContentId,
    ContentLanguage,
    ContentLocation,
    ContentTransferEncoding,
    ContentType,
    ContentDisposition,
    DkimSignature,
    ResentTo,
    ResentFrom,
    ResentBcc,
    ResentCc,
    ResentSender,
    ResentDate,
    ResentMessageId,
    ListArchive,
    ListHelp,
    ListId,
    ListOwner,
    ListPost,
    ListSubscribe,
    ListUnsubscribe,
    Other(Cow<'x, str>),
}

/// Parsed header value.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum HeaderValue<'x> {
    /// Address list or group
    Address(Address<'x>),

    /// String
    Text(Cow<'x, str>),

    /// List of strings
    TextList(Vec<Cow<'x, str>>),

    /// Datetime
    DateTime(DateTime),

    /// Content-Type or Content-Disposition header
    ContentType(ContentType<'x>),

    /// Received header
    Received(Box<Received<'x>>),

    #[default]
    Empty,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Address<'x> {
    /// Address list
    List(Vec<Addr<'x>>),
    /// Group of addresses
    Group(Vec<Group<'x>>),
}

/// Header form
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum HeaderForm {
    Raw,
    Text,
    Addresses,
    GroupedAddresses,
    MessageIds,
    Date,
    URLs,
}
/// An RFC2047 Content-Type or RFC2183 Content-Disposition MIME header field.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ContentType<'x> {
    pub c_type: Cow<'x, str>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub c_subtype: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub attributes: Option<Vec<(Cow<'x, str>, Cow<'x, str>)>>,
}

/// An RFC5322 datetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub tz_before_gmt: bool,
    pub tz_hour: u8,
    pub tz_minute: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Received<'x> {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub from: Option<Host<'x>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub from_ip: Option<IpAddr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub from_iprev: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub by: Option<Host<'x>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub for_: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub with: Option<Protocol>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub tls_version: Option<TlsVersion>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub tls_cipher: Option<Cow<'x, str>>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub id: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ident: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub helo: Option<Host<'x>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub helo_cmd: Option<Greeting>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub via: Option<Cow<'x, str>>,
    pub date: Option<DateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Host<'x> {
    Name(Cow<'x, str>),
    IpAddr(IpAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum TlsVersion {
    SSLv2,
    SSLv3,
    TLSv1_0,
    TLSv1_1,
    TLSv1_2,
    TLSv1_3,
    DTLSv1_0,
    DTLSv1_2,
    DTLSv1_3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Greeting {
    Helo,
    Ehlo,
    Lhlo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    // IANA Mail Transmission Types
    SMTP,
    ESMTP,
    ESMTPA,
    ESMTPS,
    ESMTPSA,
    LMTP,
    LMTPA,
    LMTPS,
    LMTPSA,
    MMS,
    UTF8SMTP,
    UTF8SMTPA,
    UTF8SMTPS,
    UTF8SMTPSA,
    UTF8LMTP,
    UTF8LMTPA,
    UTF8LMTPS,
    UTF8LMTPSA,

    // Non-Standard Mail Transmission Types
    HTTP,
    HTTPS,
    IMAP,
    POP3,
    Local, // includes stdin, socket, etc.
}

/// MIME Header field access trait
pub trait MimeHeaders<'x> {
    /// Returns the Content-Description field
    fn content_description(&self) -> Option<&str>;
    /// Returns the Content-Disposition field
    fn content_disposition(&self) -> Option<&ContentType<'_>>;
    /// Returns the Content-ID field
    fn content_id(&self) -> Option<&str>;
    /// Returns the Content-Encoding field
    fn content_transfer_encoding(&self) -> Option<&str>;
    /// Returns the Content-Type field
    fn content_type(&self) -> Option<&ContentType<'_>>;
    /// Returns the Content-Language field
    fn content_language(&self) -> &HeaderValue<'_>;
    /// Returns the Content-Location field
    fn content_location(&self) -> Option<&str>;
    /// Returns the attachment name, if any.
    fn attachment_name(&self) -> Option<&str> {
        self.content_disposition()
            .and_then(|cd| cd.attribute("filename"))
            .or_else(|| self.content_type().and_then(|ct| ct.attribute("name")))
    }
    // Returns true is the content type matches
    fn is_content_type(&self, type_: &str, subtype: &str) -> bool {
        self.content_type().is_some_and(|ct| {
            ct.c_type.eq_ignore_ascii_case(type_)
                && ct
                    .c_subtype
                    .as_ref()
                    .is_some_and(|st| st.eq_ignore_ascii_case(subtype))
        })
    }
}

pub trait GetHeader<'x> {
    fn header_value(&self, name: &HeaderName<'_>) -> Option<&HeaderValue<'x>>;
    fn header(&self, name: impl Into<HeaderName<'x>>) -> Option<&Header<'x>>;
}

struct BodyPartIterator<'x> {
    message: &'x Message<'x>,
    list: &'x [MessagePartId],
    pos: isize,
}

struct AttachmentIterator<'x> {
    message: &'x Message<'x>,
    pos: isize,
}
