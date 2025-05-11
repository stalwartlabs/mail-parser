/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */
#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms)]
#[forbid(unsafe_code)]
pub mod core;
pub mod decoders;
pub mod mailbox;
pub mod parsers;

use parsers::MessageStream;
use std::{borrow::Cow, collections::HashMap, hash::Hash, net::IpAddr};

/// RFC5322/RFC822 message parser.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MessageParser {
    pub(crate) header_map: HashMap<HeaderName<'static>, HdrParseFnc>,
    pub(crate) def_hdr_parse_fnc: HdrParseFnc,
}

pub(crate) type HdrParseFnc = for<'x> fn(&mut MessageStream<'x>) -> crate::HeaderValue<'x>;

/// An RFC5322/RFC822 message.
#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
pub struct Message<'x> {
    #[cfg_attr(feature = "serde", serde(default))]
    pub html_body: Vec<MessagePartId>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub text_body: Vec<MessagePartId>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub attachments: Vec<MessagePartId>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub parts: Vec<MessagePart<'x>>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Skip))]
    pub raw_message: Cow<'x, [u8]>,
}

/// MIME Message Part
#[derive(Debug, PartialEq, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
pub struct MessagePart<'x> {
    #[cfg_attr(feature = "serde", serde(default))]
    pub headers: Vec<Header<'x>>,
    pub is_encoding_problem: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    //#[cfg_attr(feature = "rkyv", rkyv(omit_bounds))]
    pub body: PartType<'x>,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub encoding: Encoding,
    pub offset_header: u32,
    pub offset_body: u32,
    pub offset_end: u32,
}

/// MIME Part encoding type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
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
pub type MessagePartId = u32;

/// A text, binary or nested e-mail MIME message part.
///
/// - Text: Any text/* part
/// - Binary: Any other part type that is not text.
/// - Message: Nested RFC5322 message.
/// - MultiPart: Multipart part.
///
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
#[cfg_attr(
    feature = "rkyv",
    rkyv(serialize_bounds(
        __S: rkyv::ser::Writer + rkyv::ser::Allocator,
        __S::Error: rkyv::rancor::Source,
    ))
)]
#[cfg_attr(
    feature = "rkyv",
    rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))
)]
#[cfg_attr(
    feature = "rkyv",
    rkyv(bytecheck(
        bounds(
            __C: rkyv::validation::ArchiveContext,
        )
    ))
)]
pub enum PartType<'x> {
    /// Any text/* part
    Text(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, str>),

    /// A text/html part
    Html(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, str>),

    /// Any other part type that is not text.
    Binary(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, [u8]>),

    /// Any inline binary data that.
    InlineBinary(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, [u8]>),

    /// Nested RFC5322 message.
    Message(#[cfg_attr(feature = "rkyv", rkyv(omit_bounds))] Message<'x>),

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]

pub struct Addr<'x> {
    /// The address name including comments
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub name: Option<Cow<'x, str>>,

    /// An e-mail address (RFC5322/RFC2369) or URL (RFC2369)
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub address: Option<Cow<'x, str>>,
}

/// An RFC5322 address group.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]

pub struct Group<'x> {
    /// Group name
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub name: Option<Cow<'x, str>>,

    /// Addresses member of the group
    #[cfg_attr(feature = "serde", serde(default))]
    pub addresses: Vec<Addr<'x>>,
}

/// A message header.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
#[cfg_attr(feature = "rkyv", rkyv(compare(PartialEq)))]
pub struct Header<'x> {
    pub name: HeaderName<'x>,
    pub value: HeaderValue<'x>,
    pub offset_field: u32,
    pub offset_start: u32,
    pub offset_end: u32,
}

/// A header field
#[derive(Debug, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
#[cfg_attr(feature = "rkyv", rkyv(compare(PartialEq)))]
#[non_exhaustive]
pub enum HeaderName<'x> {
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
    Other(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, str>),
    DkimSignature,
    ArcAuthenticationResults,
    ArcMessageSignature,
    ArcSeal,
}

/// Parsed header value.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]

pub enum HeaderValue<'x> {
    /// Address list or group
    Address(Address<'x>),

    /// String
    Text(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, str>),

    /// List of strings
    TextList(
        #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
        Vec<Cow<'x, str>>,
    ),

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
pub struct ContentType<'x> {
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))]
    pub c_type: Cow<'x, str>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub c_subtype: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Option<Vec<Attribute<'x>>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
pub struct Attribute<'x> {
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))]
    pub name: Cow<'x, str>,
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))]
    pub value: Cow<'x, str>,
}

/// An RFC5322 datetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
pub struct Received<'x> {
    #[cfg_attr(feature = "serde", serde(default))]
    pub from: Option<Host<'x>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub from_ip: Option<IpAddr>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub from_iprev: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub by: Option<Host<'x>>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub for_: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub with: Option<Protocol>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub tls_version: Option<TlsVersion>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub tls_cipher: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub id: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub ident: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub helo: Option<Host<'x>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub helo_cmd: Option<Greeting>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::Map<rkyv::with::AsOwned>))]
    pub via: Option<Cow<'x, str>>,
    pub date: Option<DateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]

pub enum Host<'x> {
    Name(#[cfg_attr(feature = "rkyv", rkyv(with = rkyv::with::AsOwned))] Cow<'x, str>),
    IpAddr(IpAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]

pub enum Greeting {
    Helo,
    Ehlo,
    Lhlo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
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
    pos: i32,
}

struct AttachmentIterator<'x> {
    message: &'x Message<'x>,
    pos: i32,
}
