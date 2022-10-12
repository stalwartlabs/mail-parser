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
//! - Every function in the library has been [fuzzed](#testing-fuzzing--benchmarking) and
//!   thoroughly [tested with MIRI](#testing-fuzzing--benchmarking).
//! - **Battle-tested** with millions of real-world e-mail messages dating from 1995 until today.
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
//!    let message = Message::parse(input).unwrap();
//!
//!    // Parses addresses (including comments), lists and groups
//!    assert_eq!(
//!        message.get_from(),
//!        &HeaderValue::Address(Addr::new(
//!            "Art Vandelay (Vandelay Industries)".into(),
//!            "art@vandelay.com"
//!        ))
//!    );
//!    
//!    assert_eq!(
//!        message.get_to(),
//!        &HeaderValue::GroupList(vec![
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
//!        ])
//!    );
//!
//!    assert_eq!(
//!        message.get_date().unwrap().to_rfc3339(),
//!        "2021-11-20T14:22:01-08:00"
//!    );
//!
//!    // RFC2047 support for encoded text in message readers
//!    assert_eq!(
//!        message.get_subject().unwrap(),
//!        "Why not both importing AND exporting? ☺"
//!    );
//!
//!    // HTML and text body parts are returned conforming to RFC8621, Section 4.1.4
//!    assert_eq!(
//!        message.get_html_body(0).unwrap(),
//!        concat!(
//!            "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
//!            "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
//!            " why not do both? &#x263A;</p></html>"
//!        )
//!    );
//!
//!    // HTML parts are converted to plain text (and viceversa) when missing
//!    assert_eq!(
//!        message.get_text_body(0).unwrap(),
//!        concat!(
//!            "I was thinking about quitting the “exporting” to focus just on the",
//!            " “importing”,\nbut then I thought, why not do both? ☺\n"
//!        )
//!    );
//!
//!    // Supports nested messages as well as multipart/digest
//!    let nested_message = message
//!        .get_attachment(0)
//!        .unwrap()
//!        .get_message()
//!        .unwrap();
//!
//!    assert_eq!(
//!        nested_message.get_subject().unwrap(),
//!        "Exporting my book about coffee tables"
//!    );
//!
//!    // Handles UTF-* as well as many legacy encodings
//!    assert_eq!(
//!        nested_message.get_text_body(0).unwrap(),
//!        "ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!"
//!    );
//!    assert_eq!(
//!        nested_message.get_html_body(0).unwrap(),
//!        "<html><body>ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!</body></html>"
//!    );
//!
//!    let nested_attachment = nested_message.get_attachment(0).unwrap();
//!
//!    assert_eq!(nested_attachment.len(), 42);
//!
//!    // Full RFC2231 support for continuations and character sets
//!    assert_eq!(
//!        nested_attachment.get_attachment_name().unwrap(),
//!        "Book about ☕ tables.gif"
//!    );
//!
//!    // Integrates with Serde
//!    println!("{}", serde_json::to_string_pretty(&message).unwrap());
//!    println!("{}", serde_yaml::to_string(&message).unwrap());
//!```
pub mod decoders;
pub mod mailbox;
pub mod parsers;

use std::{
    borrow::Cow,
    convert::TryInto,
    fmt::{self, Display},
    hash::Hash,
};

use decoders::html::{html_to_text, text_to_html};
use parsers::{
    fields::thread::thread_name,
    preview::{preview_html, preview_text},
};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum Encoding {
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

impl Default for Encoding {
    fn default() -> Self {
        Encoding::None
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

impl<'x> Addr<'x> {
    pub fn new(name: Option<&'x str>, address: &'x str) -> Self {
        Self {
            name: name.map(|name| name.into()),
            address: Some(address.into()),
        }
    }

    pub fn into_owned<'y>(self) -> Addr<'y> {
        Addr {
            name: self.name.map(|s| s.into_owned().into()),
            address: self.address.map(|s| s.into_owned().into()),
        }
    }
}

/// An RFC5322 address group.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Group<'x> {
    /// Group name
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name: Option<Cow<'x, str>>,

    /// Addressess member of the group
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub addresses: Vec<Addr<'x>>,
}

impl<'x> Group<'x> {
    pub fn new(name: &'x str, addresses: Vec<Addr<'x>>) -> Self {
        Self {
            name: Some(name.into()),
            addresses,
        }
    }

    pub fn into_owned<'y>(self) -> Group<'y> {
        Group {
            name: self.name.map(|s| s.into_owned().into()),
            addresses: self.addresses.into_iter().map(|a| a.into_owned()).collect(),
        }
    }
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

impl<'x> Header<'x> {
    /// Returns the header name
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the parsed header value
    pub fn value(&self) -> &HeaderValue {
        &self.value
    }

    /// Returns the raw offset start
    pub fn offset_start(&self) -> usize {
        self.offset_start
    }

    /// Returns the raw offset end
    pub fn offset_end(&self) -> usize {
        self.offset_end
    }

    /// Returns the raw offset of the header name
    pub fn offset_field(&self) -> usize {
        self.offset_field
    }

    /// Returns an owned version of the header
    pub fn into_owned<'y>(self) -> Header<'y> {
        Header {
            name: self.name.into_owned(),
            value: self.value.into_owned(),
            offset_field: self.offset_field,
            offset_start: self.offset_start,
            offset_end: self.offset_end,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum HeaderName<'x> {
    Rfc(RfcHeader),
    Other(Cow<'x, str>),
}

impl PartialEq for HeaderName<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rfc(a), Self::Rfc(b)) => a == b,
            (Self::Other(a), Self::Other(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        }
    }
}

impl<'x> Hash for HeaderName<'x> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            HeaderName::Rfc(rfc) => rfc.hash(state),
            HeaderName::Other(value) => {
                for ch in value.as_bytes() {
                    ch.to_ascii_lowercase().hash(state)
                }
            }
        }
    }
}

impl Eq for HeaderName<'_> {}

impl<'x> HeaderName<'x> {
    /// Parse a header name
    pub fn parse(header_name: impl Into<Cow<'x, str>>) -> Self {
        let header_name = header_name.into();
        if let Some(rfc_header) = RfcHeader::parse(header_name.as_ref()) {
            HeaderName::Rfc(rfc_header)
        } else {
            HeaderName::Other(header_name)
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            HeaderName::Rfc(header) => header.as_str(),
            HeaderName::Other(name) => name.as_ref(),
        }
    }

    pub fn as_owned<'y>(&self) -> HeaderName<'y> {
        match self {
            HeaderName::Rfc(header) => HeaderName::Rfc(*header),
            HeaderName::Other(name) => HeaderName::Other(name.clone().into_owned().into()),
        }
    }

    pub fn into_owned<'y>(self) -> HeaderName<'y> {
        match self {
            HeaderName::Rfc(header) => HeaderName::Rfc(header),
            HeaderName::Other(name) => HeaderName::Other(name.into_owned().into()),
        }
    }

    pub fn unwrap(self) -> String {
        match self {
            HeaderName::Rfc(header) => header.as_str().to_owned(),
            HeaderName::Other(name) => name.into_owned(),
        }
    }

    /// Returns true if it is a MIME header.
    pub fn is_mime_header(&self) -> bool {
        match self {
            HeaderName::Rfc(header) => header.is_mime_header(),
            HeaderName::Other(_) => false,
        }
    }

    /// Returns the lenght of the header
    pub fn len(&self) -> usize {
        match self {
            HeaderName::Rfc(name) => name.len(),
            HeaderName::Other(name) => name.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

/// A header field
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "snake_case"))]
pub enum RfcHeader {
    Subject = 0,
    From = 1,
    To = 2,
    Cc = 3,
    Date = 4,
    Bcc = 5,
    ReplyTo = 6,
    Sender = 7,
    Comments = 8,
    InReplyTo = 9,
    Keywords = 10,
    Received = 11,
    MessageId = 12,
    References = 13,
    ReturnPath = 14,
    MimeVersion = 15,
    ContentDescription = 16,
    ContentId = 17,
    ContentLanguage = 18,
    ContentLocation = 19,
    ContentTransferEncoding = 20,
    ContentType = 21,
    ContentDisposition = 22,
    ResentTo = 23,
    ResentFrom = 24,
    ResentBcc = 25,
    ResentCc = 26,
    ResentSender = 27,
    ResentDate = 28,
    ResentMessageId = 29,
    ListArchive = 30,
    ListHelp = 31,
    ListId = 32,
    ListOwner = 33,
    ListPost = 34,
    ListSubscribe = 35,
    ListUnsubscribe = 36,
} // Note: Do not add new entries without updating HDR* tables

impl RfcHeader {
    pub fn as_str(&self) -> &'static str {
        match self {
            RfcHeader::Subject => "Subject",
            RfcHeader::From => "From",
            RfcHeader::To => "To",
            RfcHeader::Cc => "Cc",
            RfcHeader::Date => "Date",
            RfcHeader::Bcc => "Bcc",
            RfcHeader::ReplyTo => "Reply-To",
            RfcHeader::Sender => "Sender",
            RfcHeader::Comments => "Comments",
            RfcHeader::InReplyTo => "In-Reply-To",
            RfcHeader::Keywords => "Keywords",
            RfcHeader::Received => "Received",
            RfcHeader::MessageId => "Message-ID",
            RfcHeader::References => "References",
            RfcHeader::ReturnPath => "Return-Path",
            RfcHeader::MimeVersion => "MIME-Version",
            RfcHeader::ContentDescription => "Content-Description",
            RfcHeader::ContentId => "Content-ID",
            RfcHeader::ContentLanguage => "Content-Language",
            RfcHeader::ContentLocation => "Content-Location",
            RfcHeader::ContentTransferEncoding => "Content-Transfer-Encoding",
            RfcHeader::ContentType => "Content-Type",
            RfcHeader::ContentDisposition => "Content-Disposition",
            RfcHeader::ResentTo => "Resent-To",
            RfcHeader::ResentFrom => "Resent-From",
            RfcHeader::ResentBcc => "Resent-Bcc",
            RfcHeader::ResentCc => "Resent-Cc",
            RfcHeader::ResentSender => "Resent-Sender",
            RfcHeader::ResentDate => "Resent-Date",
            RfcHeader::ResentMessageId => "Resent-Message-ID",
            RfcHeader::ListArchive => "List-Archive",
            RfcHeader::ListHelp => "List-Help",
            RfcHeader::ListId => "List-ID",
            RfcHeader::ListOwner => "List-Owner",
            RfcHeader::ListPost => "List-Post",
            RfcHeader::ListSubscribe => "List-Subscribe",
            RfcHeader::ListUnsubscribe => "List-Unsubscribe",
        }
    }

    pub fn len(&self) -> usize {
        match self {
            RfcHeader::Subject => "Subject".len(),
            RfcHeader::From => "From".len(),
            RfcHeader::To => "To".len(),
            RfcHeader::Cc => "Cc".len(),
            RfcHeader::Date => "Date".len(),
            RfcHeader::Bcc => "Bcc".len(),
            RfcHeader::ReplyTo => "Reply-To".len(),
            RfcHeader::Sender => "Sender".len(),
            RfcHeader::Comments => "Comments".len(),
            RfcHeader::InReplyTo => "In-Reply-To".len(),
            RfcHeader::Keywords => "Keywords".len(),
            RfcHeader::Received => "Received".len(),
            RfcHeader::MessageId => "Message-ID".len(),
            RfcHeader::References => "References".len(),
            RfcHeader::ReturnPath => "Return-Path".len(),
            RfcHeader::MimeVersion => "MIME-Version".len(),
            RfcHeader::ContentDescription => "Content-Description".len(),
            RfcHeader::ContentId => "Content-ID".len(),
            RfcHeader::ContentLanguage => "Content-Language".len(),
            RfcHeader::ContentLocation => "Content-Location".len(),
            RfcHeader::ContentTransferEncoding => "Content-Transfer-Encoding".len(),
            RfcHeader::ContentType => "Content-Type".len(),
            RfcHeader::ContentDisposition => "Content-Disposition".len(),
            RfcHeader::ResentTo => "Resent-To".len(),
            RfcHeader::ResentFrom => "Resent-From".len(),
            RfcHeader::ResentBcc => "Resent-Bcc".len(),
            RfcHeader::ResentCc => "Resent-Cc".len(),
            RfcHeader::ResentSender => "Resent-Sender".len(),
            RfcHeader::ResentDate => "Resent-Date".len(),
            RfcHeader::ResentMessageId => "Resent-Message-ID".len(),
            RfcHeader::ListArchive => "List-Archive".len(),
            RfcHeader::ListHelp => "List-Help".len(),
            RfcHeader::ListId => "List-ID".len(),
            RfcHeader::ListOwner => "List-Owner".len(),
            RfcHeader::ListPost => "List-Post".len(),
            RfcHeader::ListSubscribe => "List-Subscribe".len(),
            RfcHeader::ListUnsubscribe => "List-Unsubscribe".len(),
        }
    }

    /// Returns true if it is a MIME header.
    pub fn is_mime_header(&self) -> bool {
        matches!(
            self,
            RfcHeader::ContentDescription
                | RfcHeader::ContentId
                | RfcHeader::ContentLanguage
                | RfcHeader::ContentLocation
                | RfcHeader::ContentTransferEncoding
                | RfcHeader::ContentType
                | RfcHeader::ContentDisposition
        )
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

impl Display for RfcHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parsed header value.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum HeaderValue<'x> {
    /// Single address
    Address(Addr<'x>),

    /// Address list
    AddressList(Vec<Addr<'x>>),

    /// Group of addresses
    Group(Group<'x>),

    /// List containing two or more address groups
    GroupList(Vec<Group<'x>>),

    /// String
    Text(Cow<'x, str>),

    /// List of strings
    TextList(Vec<Cow<'x, str>>),

    /// Datetime
    DateTime(DateTime),

    /// Content-Type or Content-Disposition header
    ContentType(ContentType<'x>),

    Empty,
}

impl<'x> Default for HeaderValue<'x> {
    fn default() -> Self {
        HeaderValue::Empty
    }
}

impl<'x> HeaderValue<'x> {
    pub fn is_empty(&self) -> bool {
        *self == HeaderValue::Empty
    }

    pub fn unwrap_text(self) -> Cow<'x, str> {
        match self {
            HeaderValue::Text(s) => s,
            _ => panic!("HeaderValue::unwrap_text called on non-Text value"),
        }
    }

    pub fn unwrap_datetime(self) -> DateTime {
        match self {
            HeaderValue::DateTime(d) => d,
            _ => panic!("HeaderValue::unwrap_datetime called on non-DateTime value"),
        }
    }

    pub fn unwrap_content_type(self) -> ContentType<'x> {
        match self {
            HeaderValue::ContentType(c) => c,
            _ => panic!("HeaderValue::unwrap_content_type called on non-ContentType value"),
        }
    }

    pub fn as_text_ref(&self) -> Option<&str> {
        match *self {
            HeaderValue::Text(ref s) => Some(s),
            HeaderValue::TextList(ref l) => l.last()?.as_ref().into(),
            _ => None,
        }
    }

    pub fn as_text_list(&self) -> Option<Vec<&str>> {
        match *self {
            HeaderValue::Text(ref s) => Some(vec![s.as_ref()]),
            HeaderValue::TextList(ref l) => Some(l.iter().map(|l| l.as_ref()).collect()),
            _ => None,
        }
    }

    pub fn get_content_type(&self) -> &ContentType<'x> {
        match *self {
            HeaderValue::ContentType(ref ct) => ct,
            _ => panic!(
                "HeaderValue::get_content_type called on non-ContentType: {:?}",
                self
            ),
        }
    }

    pub fn as_content_type_ref(&self) -> Option<&ContentType> {
        match *self {
            HeaderValue::ContentType(ref c) => Some(c),
            _ => None,
        }
    }

    pub fn as_datetime_ref(&self) -> Option<&DateTime> {
        match *self {
            HeaderValue::DateTime(ref d) => Some(d),
            _ => None,
        }
    }

    pub fn into_owned<'y>(self) -> HeaderValue<'y> {
        match self {
            HeaderValue::Address(addr) => HeaderValue::Address(addr.into_owned()),
            HeaderValue::AddressList(list) => {
                HeaderValue::AddressList(list.into_iter().map(|addr| addr.into_owned()).collect())
            }
            HeaderValue::Group(group) => HeaderValue::Group(group.into_owned()),
            HeaderValue::GroupList(list) => {
                HeaderValue::GroupList(list.into_iter().map(|group| group.into_owned()).collect())
            }
            HeaderValue::Text(text) => HeaderValue::Text(text.into_owned().into()),
            HeaderValue::TextList(list) => HeaderValue::TextList(
                list.into_iter()
                    .map(|text| text.into_owned().into())
                    .collect(),
            ),
            HeaderValue::DateTime(datetime) => HeaderValue::DateTime(datetime),
            HeaderValue::ContentType(ct) => HeaderValue::ContentType(ContentType {
                c_type: ct.c_type.into_owned().into(),
                c_subtype: ct.c_subtype.map(|s| s.into_owned().into()),
                attributes: ct.attributes.map(|attributes| {
                    attributes
                        .into_iter()
                        .map(|(k, v)| (k.into_owned().into(), v.into_owned().into()))
                        .collect()
                }),
            }),
            HeaderValue::Empty => HeaderValue::Empty,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            HeaderValue::Text(text) => text.len(),
            HeaderValue::TextList(list) => list.iter().map(|t| t.len()).sum(),
            HeaderValue::Address(a) => {
                a.name.as_ref().map_or(0, |a| a.len()) + a.address.as_ref().map_or(0, |a| a.len())
            }
            HeaderValue::AddressList(list) => list
                .iter()
                .map(|a| {
                    a.name.as_ref().map_or(0, |a| a.len())
                        + a.address.as_ref().map_or(0, |a| a.len())
                })
                .sum(),
            HeaderValue::Group(group) => group
                .addresses
                .iter()
                .map(|a| {
                    a.name.as_ref().map_or(0, |a| a.len())
                        + a.address.as_ref().map_or(0, |a| a.len())
                })
                .sum(),
            HeaderValue::GroupList(grouplist) => grouplist
                .iter()
                .flat_map(|g| g.addresses.iter())
                .map(|a| {
                    a.name.as_ref().map_or(0, |a| a.len())
                        + a.address.as_ref().map_or(0, |a| a.len())
                })
                .sum(),
            HeaderValue::DateTime(_) => 24,
            HeaderValue::ContentType(ct) => {
                ct.c_type.len()
                    + ct.c_subtype.as_ref().map_or(0, |s| s.len())
                    + ct.attributes
                        .as_ref()
                        .map_or(0, |at| at.iter().map(|(a, b)| a.len() + b.len()).sum())
            }
            HeaderValue::Empty => 0,
        }
    }
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl<'x> Message<'x> {
    /// Returns the root message part
    pub fn get_root_part(&self) -> &MessagePart<'x> {
        &self.parts[0]
    }

    /// Returns a parsed header.
    pub fn get_header(&self, header: &str) -> Option<&HeaderValue> {
        self.parts[0].headers.get_header(header).map(|h| &h.value)
    }

    /// Removed a parsed header and returns its value.
    pub fn remove_header(&mut self, header: &str) -> Option<HeaderValue> {
        let headers = &mut self.parts[0].headers;
        headers
            .iter()
            .position(|h| h.name.as_str() == header)
            .map(|pos| headers.swap_remove(pos).value)
    }

    /// Removed a parsed RFC heade and returns its value.
    pub fn remove_header_rfc(&mut self, header: RfcHeader) -> Option<HeaderValue> {
        let headers = &mut self.parts[0].headers;
        headers
            .iter()
            .position(|h| matches!(&h.name, HeaderName::Rfc(header_name) if header_name == &header))
            .map(|pos| headers.swap_remove(pos).value)
    }

    /// Returns the raw header.
    pub fn get_header_raw(&self, header: &str) -> Option<&str> {
        self.parts[0]
            .headers
            .get_header(header)
            .and_then(|h| std::str::from_utf8(&self.raw_message[h.offset_start..h.offset_end]).ok())
    }

    /// Returns an iterator over the RFC headers of this message.
    pub fn get_headers(&self) -> &[Header] {
        &self.parts[0].headers
    }

    /// Returns an iterator over the matching RFC headers of this message.
    pub fn get_header_values<'y: 'x>(
        &'y self,
        name: RfcHeader,
    ) -> impl Iterator<Item = &HeaderValue<'x>> {
        self.parts[0].headers.iter().filter_map(move |header| {
            if matches!(&header.name, HeaderName::Rfc(rfc_name) if rfc_name == &name) {
                Some(&header.value)
            } else {
                None
            }
        })
    }

    /// Returns all headers in raw format
    pub fn get_headers_raw(&self) -> impl Iterator<Item = (&str, &str)> {
        self.parts[0].headers.iter().filter_map(move |header| {
            Some((
                header.name.as_str(),
                std::str::from_utf8(&self.raw_message[header.offset_start..header.offset_end])
                    .ok()?,
            ))
        })
    }

    /// Returns the BCC header field
    pub fn get_bcc(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Bcc)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the CC header field
    pub fn get_cc(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Cc)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Comments header fields
    pub fn get_comments(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Comments)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Date header field
    pub fn get_date(&self) -> Option<&DateTime> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Date)
            .and_then(|header| header.as_datetime_ref())
    }

    /// Returns the From header field
    pub fn get_from(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::From)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all In-Reply-To header fields
    pub fn get_in_reply_to(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::InReplyTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Keywords header fields
    pub fn get_keywords(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Keywords)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Archive header field
    pub fn get_list_archive(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListArchive)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Help header field
    pub fn get_list_help(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListHelp)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-ID header field
    pub fn get_list_id(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListId)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Owner header field
    pub fn get_list_owner(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListOwner)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Post header field
    pub fn get_list_post(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListPost)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Subscribe header field
    pub fn get_list_subscribe(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListSubscribe)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Unsubscribe header field
    pub fn get_list_unsubscribe(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ListUnsubscribe)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Message-ID header field
    pub fn get_message_id(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::MessageId)
            .and_then(|header| header.as_text_ref())
    }

    /// Returns the MIME-Version header field
    pub fn get_mime_version(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::MimeVersion)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Received header fields
    pub fn get_received(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Received)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all References header fields
    pub fn get_references(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::References)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Reply-To header field
    pub fn get_reply_to(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ReplyTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-BCC header field
    pub fn get_resent_bcc(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentBcc)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-CC header field
    pub fn get_resent_cc(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Resent-Date header fields
    pub fn get_resent_date(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentDate)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-From header field
    pub fn get_resent_from(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentFrom)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Resent-Message-ID header fields
    pub fn get_resent_message_id(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentMessageId)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Sender header field
    pub fn get_resent_sender(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentSender)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-To header field
    pub fn get_resent_to(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ResentTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Return-Path header fields
    pub fn get_return_path(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ReturnPath)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the return address from either the Return-Path
    /// or From header fields
    pub fn get_return_address(&self) -> Option<&str> {
        match self.parts[0].headers.get_rfc(&RfcHeader::ReturnPath) {
            Some(HeaderValue::Text(text)) => Some(text.as_ref()),
            Some(HeaderValue::TextList(text_list)) => text_list.last().map(|t| t.as_ref()),
            _ => match self.parts[0].headers.get_rfc(&RfcHeader::From) {
                Some(HeaderValue::Address(addr)) => addr.address.as_deref(),
                Some(HeaderValue::AddressList(addr_list)) => {
                    addr_list.last().and_then(|a| a.address.as_deref())
                }
                _ => None,
            },
        }
    }

    /// Returns the Sender header field
    pub fn get_sender(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Sender)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Subject header field
    pub fn get_subject(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::Subject)
            .and_then(|header| header.as_text_ref())
    }

    /// Returns the message thread name or 'base subject' as defined in
    /// [RFC 5957 - Internet Message Access Protocol - SORT and THREAD Extensions (Section 2.1)](https://datatracker.ietf.org/doc/html/rfc5256#section-2.1)
    pub fn get_thread_name(&self) -> Option<&str> {
        thread_name(self.get_subject()?).into()
    }

    /// Returns the To header field
    pub fn get_to(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::To)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns a preview of the message body
    pub fn get_body_preview(&self, preview_len: usize) -> Option<Cow<'x, str>> {
        if !self.text_body.is_empty() {
            preview_text(self.get_text_body(0)?, preview_len).into()
        } else if !self.html_body.is_empty() {
            preview_html(self.get_html_body(0)?, preview_len).into()
        } else {
            None
        }
    }

    /// Returns the transformed contents an inline HTML body part by position
    pub fn get_html_body(&'x self, pos: usize) -> Option<Cow<'x, str>> {
        let part = self.parts.get(*self.html_body.get(pos)?)?;
        match &part.body {
            PartType::Html(html) => Some(html.as_ref().into()),
            PartType::Text(text) => Some(text_to_html(text.as_ref()).into()),
            _ => None,
        }
    }

    /// Returns the transformed contents an inline text body part by position
    pub fn get_text_body(&'x self, pos: usize) -> Option<Cow<'x, str>> {
        let part = self.parts.get(*self.html_body.get(pos)?)?;
        match &part.body {
            PartType::Text(text) => Some(text.as_ref().into()),
            PartType::Html(html) => Some(html_to_text(html.as_ref()).into()),
            _ => None,
        }
    }

    /// Returns a message part by position
    pub fn get_part(&self, pos: usize) -> Option<&MessagePart> {
        self.parts.get(pos)
    }

    /// Returns an inline HTML body part by position
    pub fn get_html_part(&self, pos: usize) -> Option<&MessagePart> {
        self.parts.get(*self.html_body.get(pos)?)
    }

    /// Returns an inline text body part by position
    pub fn get_text_part(&self, pos: usize) -> Option<&MessagePart> {
        self.parts.get(*self.text_body.get(pos)?)
    }

    /// Returns an attacment by position
    pub fn get_attachment(&self, pos: usize) -> Option<&MessagePart<'x>> {
        self.parts.get(*self.attachments.get(pos)?)
    }

    /// Returns the number of plain text body parts
    pub fn get_text_body_count(&self) -> usize {
        self.text_body.len()
    }

    /// Returns the number of HTML body parts
    pub fn get_html_body_count(&self) -> usize {
        self.html_body.len()
    }

    /// Returns the number of attachments
    pub fn get_attachment_count(&self) -> usize {
        self.attachments.len()
    }

    /// Returns an Interator over the text body parts
    pub fn get_text_bodies(&'x self) -> BodyPartIterator<'x> {
        BodyPartIterator::new(self, &self.text_body)
    }

    /// Returns an Interator over the HTML body parts
    pub fn get_html_bodies(&'x self) -> BodyPartIterator<'x> {
        BodyPartIterator::new(self, &self.html_body)
    }

    /// Returns an Interator over the attachments
    pub fn get_attachments(&'x self) -> AttachmentIterator<'x> {
        AttachmentIterator::new(self)
    }

    /// Returns an owned version of the message
    pub fn into_owned<'y>(self) -> Message<'y> {
        Message {
            html_body: self.html_body,
            text_body: self.text_body,
            attachments: self.attachments,
            parts: self.parts.into_iter().map(|p| p.into_owned()).collect(),
            raw_message: self.raw_message.into_owned().into(),
        }
    }
}

/// MIME Header field access trait
pub trait MimeHeaders<'x> {
    /// Returns the Content-Description field
    fn get_content_description(&self) -> Option<&str>;
    /// Returns the Content-Disposition field
    fn get_content_disposition(&self) -> Option<&ContentType>;
    /// Returns the Content-ID field
    fn get_content_id(&self) -> Option<&str>;
    /// Returns the Content-Encoding field
    fn get_content_transfer_encoding(&self) -> Option<&str>;
    /// Returns the Content-Type field
    fn get_content_type(&self) -> Option<&ContentType>;
    /// Returns the Content-Language field
    fn get_content_language(&self) -> &HeaderValue;
    /// Returns the Content-Location field
    fn get_content_location(&self) -> Option<&str>;
    /// Returns the attachment name, if any.
    fn get_attachment_name(&self) -> Option<&str> {
        self.get_content_disposition()
            .and_then(|cd| cd.get_attribute("filename"))
            .or_else(|| {
                self.get_content_type()
                    .and_then(|ct| ct.get_attribute("name"))
            })
    }
}

impl<'x> MimeHeaders<'x> for Message<'x> {
    fn get_content_description(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentDescription)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_disposition(&self) -> Option<&ContentType> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentDisposition)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_id(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentId)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_transfer_encoding(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentTransferEncoding)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_type(&self) -> Option<&ContentType> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentType)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_language(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentLanguage)
            .unwrap_or(&HeaderValue::Empty)
    }

    fn get_content_location(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .get_rfc(&RfcHeader::ContentLocation)
            .and_then(|header| header.as_text_ref())
    }
}

impl<'x> MessagePart<'x> {
    /// Returns the body part's contents as a `u8` slice
    pub fn get_contents(&'x self) -> &'x [u8] {
        match &self.body {
            PartType::Text(text) | PartType::Html(text) => text.as_bytes(),
            PartType::Binary(bin) | PartType::InlineBinary(bin) => bin.as_ref(),
            PartType::Message(message) => message.raw_message.as_ref(),
            PartType::Multipart(_) => b"",
        }
    }

    /// Returns the body part's contents as a `str`
    pub fn get_text_contents(&'x self) -> Option<&'x str> {
        match &self.body {
            PartType::Text(text) | PartType::Html(text) => text.as_ref().into(),
            PartType::Binary(bin) | PartType::InlineBinary(bin) => {
                std::str::from_utf8(bin.as_ref()).ok()
            }
            PartType::Message(message) => std::str::from_utf8(message.raw_message.as_ref()).ok(),
            PartType::Multipart(_) => None,
        }
    }

    /// Returns the nested message
    pub fn get_message(&'x self) -> Option<&Message<'x>> {
        if let PartType::Message(message) = &self.body {
            Some(message)
        } else {
            None
        }
    }

    /// Returns the sub parts ids of a MIME part
    pub fn get_sub_parts(&'x self) -> Option<&[MessagePartId]> {
        if let PartType::Multipart(parts) = &self.body {
            Some(parts.as_ref())
        } else {
            None
        }
    }

    /// Returns the body part's length
    pub fn len(&self) -> usize {
        match &self.body {
            PartType::Text(text) | PartType::Html(text) => text.len(),
            PartType::Binary(bin) | PartType::InlineBinary(bin) => bin.len(),
            PartType::Message(message) => message.raw_message.len(),
            PartType::Multipart(_) => 0,
        }
    }

    /// Returns `true` when the body part MIME type is text/*
    pub fn is_text(&self) -> bool {
        matches!(self.body, PartType::Text(_) | PartType::Html(_))
    }

    /// Returns `true` when the body part MIME type is text/tml
    pub fn is_text_html(&self) -> bool {
        matches!(self.body, PartType::Html(_))
    }

    /// Returns `true` when the part is binary
    pub fn is_binary(&self) -> bool {
        matches!(self.body, PartType::Binary(_) | PartType::InlineBinary(_))
    }

    /// Returns `true` when the part is multipart
    pub fn is_multipart(&self) -> bool {
        matches!(self.body, PartType::Multipart(_))
    }

    /// Returns `true` when the part is a nested message
    pub fn is_message(&self) -> bool {
        matches!(self.body, PartType::Message(_))
    }

    /// Returns `true` when the body part is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the message headers
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    /// Returns the body raw length
    pub fn raw_len(&self) -> usize {
        self.offset_end.saturating_sub(self.offset_header)
    }

    /// Get the raw header offset of this part
    pub fn raw_header_offset(&self) -> usize {
        self.offset_header
    }

    /// Get the raw body offset of this part
    pub fn raw_body_offset(&self) -> usize {
        self.offset_body
    }

    /// Get the raw body end offset of this part
    pub fn raw_end_offset(&self) -> usize {
        self.offset_end
    }

    /// Returns an owned version of the this part
    pub fn into_owned<'y>(self) -> MessagePart<'y> {
        MessagePart {
            headers: self.headers.into_iter().map(|h| h.into_owned()).collect(),
            is_encoding_problem: self.is_encoding_problem,
            body: match self.body {
                PartType::Text(v) => PartType::Text(v.into_owned().into()),
                PartType::Html(v) => PartType::Html(v.into_owned().into()),
                PartType::Binary(v) => PartType::Binary(v.into_owned().into()),
                PartType::InlineBinary(v) => PartType::InlineBinary(v.into_owned().into()),
                PartType::Message(v) => PartType::Message(v.into_owned()),
                PartType::Multipart(v) => PartType::Multipart(v),
            },
            encoding: self.encoding,
            offset_header: self.offset_header,
            offset_body: self.offset_body,
            offset_end: self.offset_end,
        }
    }
}

impl<'x> fmt::Display for MessagePart<'x> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.get_text_contents().unwrap_or("[no contents]"))
    }
}

impl<'x> MimeHeaders<'x> for MessagePart<'x> {
    fn get_content_description(&self) -> Option<&str> {
        self.headers
            .get_rfc(&RfcHeader::ContentDescription)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_disposition(&self) -> Option<&ContentType> {
        self.headers
            .get_rfc(&RfcHeader::ContentDisposition)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_id(&self) -> Option<&str> {
        self.headers
            .get_rfc(&RfcHeader::ContentId)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_transfer_encoding(&self) -> Option<&str> {
        self.headers
            .get_rfc(&RfcHeader::ContentTransferEncoding)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_type(&self) -> Option<&ContentType> {
        self.headers
            .get_rfc(&RfcHeader::ContentType)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_language(&self) -> &HeaderValue {
        self.headers
            .get_rfc(&RfcHeader::ContentLanguage)
            .unwrap_or(&HeaderValue::Empty)
    }

    fn get_content_location(&self) -> Option<&str> {
        self.headers
            .get_rfc(&RfcHeader::ContentLocation)
            .and_then(|header| header.as_text_ref())
    }
}

pub trait GetHeader {
    fn get_rfc(&self, name: &RfcHeader) -> Option<&HeaderValue>;
    fn get_header(&self, name: &str) -> Option<&Header>;
}

impl<'x> GetHeader for Vec<Header<'x>> {
    fn get_rfc(&self, name: &RfcHeader) -> Option<&HeaderValue<'x>> {
        self.iter()
            .rev()
            .find(|header| matches!(&header.name, HeaderName::Rfc(rfc_name) if rfc_name == name))
            .map(|header| &header.value)
    }

    fn get_header(&self, name: &str) -> Option<&Header> {
        self.iter()
            .rev()
            .find(|header| header.name.as_str().eq_ignore_ascii_case(name))
    }
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

impl<'x> BodyPartIterator<'x> {
    fn new(message: &'x Message<'x>, list: &'x [MessagePartId]) -> BodyPartIterator<'x> {
        BodyPartIterator {
            message,
            list,
            pos: -1,
        }
    }
}

impl<'x> Iterator for BodyPartIterator<'x> {
    type Item = &'x MessagePart<'x>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.message.parts.get(*self.list.get(self.pos as usize)?)
    }
}

impl<'x> AttachmentIterator<'x> {
    fn new(message: &'x Message<'x>) -> AttachmentIterator<'x> {
        AttachmentIterator { message, pos: -1 }
    }
}

impl<'x> Iterator for AttachmentIterator<'x> {
    type Item = &'x MessagePart<'x>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.message.get_attachment(self.pos as usize)
    }
}

/// An RFC2047 Content-Type or RFC2183 Content-Disposition MIME header field.
impl<'x> ContentType<'x> {
    /// Returns the type
    pub fn get_type(&self) -> &str {
        &self.c_type
    }

    /// Returns the sub-type
    pub fn get_subtype(&self) -> Option<&str> {
        self.c_subtype.as_ref()?.as_ref().into()
    }

    /// Returns an attribute by name
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .as_ref()?
            .iter()
            .find(|(key, _)| key == name)?
            .1
            .as_ref()
            .into()
    }

    /// Removes an attribute by name
    pub fn remove_attribute(&mut self, name: &str) -> Option<Cow<str>> {
        let attributes = self.attributes.as_mut()?;

        attributes
            .iter()
            .position(|(key, _)| key == name)
            .map(|pos| attributes.swap_remove(pos).1)
    }

    /// Returns all attributes
    pub fn get_attributes(&self) -> Option<&[(Cow<str>, Cow<str>)]> {
        self.attributes.as_deref()
    }

    /// Returns `true` when the provided attribute name is present
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes
            .as_ref()
            .map_or_else(|| false, |attr| attr.iter().any(|(key, _)| key == name))
    }

    /// Returns ```true``` if the Content-Disposition type is "attachment"
    pub fn is_attachment(&self) -> bool {
        self.c_type.eq_ignore_ascii_case("attachment")
    }

    /// Returns ```true``` if the Content-Disposition type is "inline"
    pub fn is_inline(&self) -> bool {
        self.c_type.eq_ignore_ascii_case("inline")
    }
}

impl From<DateTime> for i64 {
    fn from(value: DateTime) -> Self {
        value.to_timestamp()
    }
}

impl From<RfcHeader> for String {
    fn from(header: RfcHeader) -> Self {
        header.to_string()
    }
}

impl From<RfcHeader> for Cow<'_, str> {
    fn from(header: RfcHeader) -> Self {
        Cow::Borrowed(header.as_str())
    }
}

impl<'x> TryInto<Message<'x>> for &'x [u8] {
    type Error = ();

    fn try_into(self) -> Result<Message<'x>, Self::Error> {
        Message::parse(self).ok_or(())
    }
}
