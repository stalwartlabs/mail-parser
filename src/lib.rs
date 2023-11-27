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

//! # mail-parser
//!
//! _mail-parser_ is an **e-mail parsing library** written in Rust that fully conforms to the Internet Message Format standard (_RFC 5322_), the
//! Multipurpose Internet Mail Extensions (MIME; _RFC 2045 - 2049_) as well as many other [internet messaging RFCs](#conformed-rfcs).
//!
//! It also supports decoding messages in [41 different character sets](#supported-character-sets) including obsolete formats such as UTF-7.
//! All Unicode (UTF-*) and single-byte character sets are handled internally by the library while support for legacy multi-byte encodings of Chinese
//! and Japanese languages such as BIG5 or ISO-2022-JP is provided by the optional dependency [encoding_rs](https://crates.io/crates/encoding_rs).
//!
//! In general, this library abides by the Postel's law or [Robustness Principle](https://en.wikipedia.org/wiki/Robustness_principle) which
//! states that an implementation must be conservative in its sending behavior and liberal in its receiving behavior. This means that
//! _mail-parser_ will make a best effort to parse non-conformant e-mail messages as long as these do not deviate too much from the standard.
//!
//! Unlike other e-mail parsing libraries that return nested representations of the different MIME parts in a message, this library
//! conforms to [RFC 8621, Section 4.1.4](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4) and provides a more human-friendly
//! representation of the message contents consisting of just text body parts, html body parts and attachments. Additionally, conversion to/from
//! HTML and plain text inline body parts is done automatically when the _alternative_ version is missing.
//!
//! Performance and memory safety were two important factors while designing _mail-parser_:
//!
//! - **Zero-copy**: Practically all strings returned by this library are `Cow<str>` references to the input raw message.
//! - **High performance Base64 decoding** based on Chromium's decoder ([the fastest non-SIMD decoder](https://github.com/lemire/fastbase64)).
//! - **Fast parsing** of message header fields, character set names and HTML entities using [perfect hashing](https://en.wikipedia.org/wiki/Perfect_hash_function).
//! - Written in **100% safe** Rust with no external dependencies.
//! - Every function in the library has been [fuzzed](#testing-fuzzing--benchmarking) and thoroughly [tested with MIRI](#testing-fuzzing--benchmarking).
//! - **Battle-tested** with millions of real-world e-mail messages dating from 1995 until today.
//! - Used in production environments worldwide by [Stalwart Mail Server](https://github.com/stalwartlabs/mail-server).
//!
//! Jump to the [example](#usage-example).
//!
//! ## Conformed RFCs
//!
//! - [RFC 822 - Standard for ARPA Internet Text Messages](https://datatracker.ietf.org/doc/html/rfc822)
//! - [RFC 5322 - Internet Message Format](https://datatracker.ietf.org/doc/html/rfc5322)
//! - [RFC 2045 - Multipurpose Internet Mail Extensions (MIME) Part One: Format of Internet Message Bodies](https://datatracker.ietf.org/doc/html/rfc2045)
//! - [RFC 2046 - Multipurpose Internet Mail Extensions (MIME) Part Two: Media Types](https://datatracker.ietf.org/doc/html/rfc2046)
//! - [RFC 2047 - MIME (Multipurpose Internet Mail Extensions) Part Three: Message Header Extensions for Non-ASCII Text](https://datatracker.ietf.org/doc/html/rfc2047)
//! - [RFC 2048 - Multipurpose Internet Mail Extensions (MIME) Part Four: Registration Procedures](https://datatracker.ietf.org/doc/html/rfc2048)
//! - [RFC 2049 - Multipurpose Internet Mail Extensions (MIME) Part Five: Conformance Criteria and Examples](https://datatracker.ietf.org/doc/html/rfc2049)
//! - [RFC 2231 - MIME Parameter Value and Encoded Word Extensions: Character Sets, Languages, and Continuations](https://datatracker.ietf.org/doc/html/rfc2231)
//! - [RFC 2557 - MIME Encapsulation of Aggregate Documents, such as HTML (MHTML)](https://datatracker.ietf.org/doc/html/rfc2557)
//! - [RFC 2183 - Communicating Presentation Information in Internet Messages: The Content-Disposition Header Field](https://datatracker.ietf.org/doc/html/rfc2183)
//! - [RFC 2392 - Content-ID and Message-ID Uniform Resource Locators](https://datatracker.ietf.org/doc/html/rfc2392)
//! - [RFC 3282 - Content Language Headers](https://datatracker.ietf.org/doc/html/rfc3282)
//! - [RFC 6532 - Internationalized Email Headers](https://datatracker.ietf.org/doc/html/rfc6532)
//! - [RFC 2152 - UTF-7 - A Mail-Safe Transformation Format of Unicode](https://datatracker.ietf.org/doc/html/rfc2152)
//! - [RFC 2369 - The Use of URLs as Meta-Syntax for Core Mail List Commands and their Transport through Message Header Fields](https://datatracker.ietf.org/doc/html/rfc2369)
//! - [RFC 2919 - List-Id: A Structured Field and Namespace for the Identification of Mailing Lists](https://datatracker.ietf.org/doc/html/rfc2919)
//! - [RFC 3339 - Date and Time on the Internet: Timestamps](https://datatracker.ietf.org/doc/html/rfc3339)
//! - [RFC 8621 - The JSON Meta Application Protocol (JMAP) for Mail (Section 4.1.4)](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4)
//! - [RFC 5957 - Internet Message Access Protocol - SORT and THREAD Extensions (Section 2.1)](https://datatracker.ietf.org/doc/html/rfc5256#section-2.1)
//!
//! ## Supported Character Sets
//!
//! - UTF-8
//! - UTF-16, UTF-16BE, UTF-16LE
//! - UTF-7
//! - US-ASCII
//! - ISO-8859-1
//! - ISO-8859-2
//! - ISO-8859-3
//! - ISO-8859-4
//! - ISO-8859-5
//! - ISO-8859-6
//! - ISO-8859-7
//! - ISO-8859-8
//! - ISO-8859-9
//! - ISO-8859-10
//! - ISO-8859-13
//! - ISO-8859-14
//! - ISO-8859-15
//! - ISO-8859-16
//! - CP1250
//! - CP1251
//! - CP1252
//! - CP1253
//! - CP1254
//! - CP1255
//! - CP1256
//! - CP1257
//! - CP1258
//! - KOI8-R
//! - KOI8_U
//! - MACINTOSH
//! - IBM850
//! - TIS-620
//!
//! Supported character sets via the optional dependency [encoding_rs](https://crates.io/crates/encoding_rs):
//!   
//! - SHIFT_JIS
//! - BIG5
//! - EUC-JP
//! - EUC-KR
//! - GB18030
//! - GBK
//! - ISO-2022-JP
//! - WINDOWS-874
//! - IBM-866
//!
//! ## Usage Example
//!
//! ```
//!    use mail_parser::*;
//!
//!    let input = br#"From: Art Vandelay <art@vandelay.com> (Vandelay Industries)
//!To: "Colleagues": "James Smythe" <james@vandelay.com>; Friends:
//!    jane@example.com, =?UTF-8?Q?John_Sm=C3=AEth?= <john@example.com>;
//!Date: Sat, 20 Nov 2021 14:22:01 -0800
//!Subject: Why not both importing AND exporting? =?utf-8?b?4pi6?=
//!Content-Type: multipart/mixed; boundary="festivus";
//!
//!--festivus
//!Content-Type: text/html; charset="us-ascii"
//!Content-Transfer-Encoding: base64
//!
//!PGh0bWw+PHA+SSB3YXMgdGhpbmtpbmcgYWJvdXQgcXVpdHRpbmcgdGhlICZsZHF1bztle
//!HBvcnRpbmcmcmRxdW87IHRvIGZvY3VzIGp1c3Qgb24gdGhlICZsZHF1bztpbXBvcnRpbm
//!cmcmRxdW87LDwvcD48cD5idXQgdGhlbiBJIHRob3VnaHQsIHdoeSBub3QgZG8gYm90aD8
//!gJiN4MjYzQTs8L3A+PC9odG1sPg==
//!--festivus
//!Content-Type: message/rfc822
//!
//!From: "Cosmo Kramer" <kramer@kramerica.com>
//!Subject: Exporting my book about coffee tables
//!Content-Type: multipart/mixed; boundary="giddyup";
//!
//!--giddyup
//!Content-Type: text/plain; charset="utf-16"
//!Content-Transfer-Encoding: quoted-printable
//!
//!=FF=FE=0C!5=D8"=DD5=D8)=DD5=D8-=DD =005=D8*=DD5=D8"=DD =005=D8"=
//!=DD5=D85=DD5=D8-=DD5=D8,=DD5=D8/=DD5=D81=DD =005=D8*=DD5=D86=DD =
//!=005=D8=1F=DD5=D8,=DD5=D8,=DD5=D8(=DD =005=D8-=DD5=D8)=DD5=D8"=
//!=DD5=D8=1E=DD5=D80=DD5=D8"=DD!=00
//!--giddyup
//!Content-Type: image/gif; name*1="about "; name*0="Book ";
//!              name*2*=utf-8''%e2%98%95 tables.gif
//!Content-Transfer-Encoding: Base64
//!Content-Disposition: attachment
//!
//!R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7
//!--giddyup--
//!--festivus--
//!"#;
//!
//!    let message = MessageParser::default().parse(input).unwrap();
//!
//!    // Parses addresses (including comments), lists and groups
//!    assert_eq!(
//!        message.from().unwrap().first().unwrap(),
//!        &Addr::new(
//!            "Art Vandelay (Vandelay Industries)".into(),
//!            "art@vandelay.com"
//!        )
//!    );
//!    
//!    assert_eq!(
//!        message.to().unwrap().as_group().unwrap(),
//!        &[
//!            Group::new(
//!                "Colleagues",
//!                vec![Addr::new("James Smythe".into(), "james@vandelay.com")]
//!            ),
//!            Group::new(
//!                "Friends",
//!                vec![
//!                    Addr::new(None, "jane@example.com"),
//!                    Addr::new("John Smîth".into(), "john@example.com"),
//!                ]
//!            )
//!        ]
//!    );
//!
//!    assert_eq!(
//!        message.date().unwrap().to_rfc3339(),
//!        "2021-11-20T14:22:01-08:00"
//!    );
//!
//!    // RFC2047 support for encoded text in message readers
//!    assert_eq!(
//!        message.subject().unwrap(),
//!        "Why not both importing AND exporting? ☺"
//!    );
//!
//!    // HTML and text body parts are returned conforming to RFC8621, Section 4.1.4
//!    assert_eq!(
//!        message.body_html(0).unwrap(),
//!        concat!(
//!            "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
//!            "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
//!            " why not do both? &#x263A;</p></html>"
//!        )
//!    );
//!
//!    // HTML parts are converted to plain text (and viceversa) when missing
//!    assert_eq!(
//!        message.body_text(0).unwrap(),
//!        concat!(
//!            "I was thinking about quitting the “exporting” to focus just on the",
//!            " “importing”,\nbut then I thought, why not do both? ☺\n"
//!        )
//!    );
//!
//!    // Supports nested messages as well as multipart/digest
//!    let nested_message = message
//!        .attachment(0)
//!        .unwrap()
//!        .message()
//!        .unwrap();
//!
//!    assert_eq!(
//!        nested_message.subject().unwrap(),
//!        "Exporting my book about coffee tables"
//!    );
//!
//!    // Handles UTF-* as well as many legacy encodings
//!    assert_eq!(
//!        nested_message.body_text(0).unwrap(),
//!        "ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!"
//!    );
//!    assert_eq!(
//!        nested_message.body_html(0).unwrap(),
//!        "<html><body>ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!</body></html>"
//!    );
//!
//!    let nested_attachment = nested_message.attachment(0).unwrap();
//!
//!    assert_eq!(nested_attachment.len(), 42);
//!
//!    // Full RFC2231 support for continuations and character sets
//!    assert_eq!(
//!        nested_attachment.attachment_name().unwrap(),
//!        "Book about ☕ tables.gif"
//!    );
//!
//!    // Integrates with Serde
//!    println!("{}", serde_json::to_string_pretty(&message).unwrap());
//!```
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
    #[cfg_attr(feature = "serde_support", serde(borrow))]
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
    #[cfg_attr(feature = "serde_support", serde(borrow))]
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
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    Binary(Cow<'x, [u8]>),

    /// Any inline binary data that.
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    InlineBinary(Cow<'x, [u8]>),

    /// Nested RFC5322 message.
    Message(Message<'x>),

    /// Multipart part
    Multipart(Vec<MessagePartId>),
}

impl<'x> Default for PartType<'x> {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    fn content_disposition(&self) -> Option<&ContentType>;
    /// Returns the Content-ID field
    fn content_id(&self) -> Option<&str>;
    /// Returns the Content-Encoding field
    fn content_transfer_encoding(&self) -> Option<&str>;
    /// Returns the Content-Type field
    fn content_type(&self) -> Option<&ContentType>;
    /// Returns the Content-Language field
    fn content_language(&self) -> &HeaderValue;
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
        self.content_type().map_or(false, |ct| {
            ct.c_type.eq_ignore_ascii_case(type_)
                && ct
                    .c_subtype
                    .as_ref()
                    .map_or(false, |st| st.eq_ignore_ascii_case(subtype))
        })
    }
}

pub trait GetHeader<'x> {
    fn header_value(&self, name: &HeaderName) -> Option<&HeaderValue>;
    fn header(&self, name: impl Into<HeaderName<'x>>) -> Option<&Header>;
}

#[doc(hidden)]
pub struct BodyPartIterator<'x> {
    message: &'x Message<'x>,
    list: &'x [MessagePartId],
    pos: isize,
}

#[doc(hidden)]
pub struct AttachmentIterator<'x> {
    message: &'x Message<'x>,
    pos: isize,
}
