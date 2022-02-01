/*
 * Copyright Stalwart Labs, Minter Ltd. See the COPYING
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
//!        message.get_date().unwrap().to_iso8601(),
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
//!        .unwrap_message();
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
//!    let nested_attachment = nested_message.get_attachment(0).unwrap().unwrap_binary();
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
#[forbid(unsafe_code)]
pub mod decoders;
pub mod parsers;

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Display},
};

use decoders::html::{html_to_text, text_to_html};
use parsers::{
    fields::thread::thread_name,
    preview::{preview_html, preview_text},
};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// An RFC5322/RFC822 message.
#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Message<'x> {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub headers_rfc: RfcHeaders<'x>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub headers_raw: RawHeaders<'x>,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub html_body: Vec<MessagePartId>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub text_body: Vec<MessagePartId>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub attachments: Vec<MessagePartId>,

    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    pub parts: Vec<MessagePart<'x>>,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub structure: MessageStructure,

    pub offset_header: usize,
    pub offset_body: usize,
    pub offset_end: usize,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub raw_message: Cow<'x, [u8]>,
}

/// Body structure.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum MessageStructure {
    Part(MessagePartId),
    List(Vec<MessageStructure>),
    MultiPart((MessagePartId, Vec<MessageStructure>)),
}

impl Default for MessageStructure {
    fn default() -> Self {
        MessageStructure::Part(0)
    }
}

/// Part of the message.
#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Part<'x, T> {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub headers_rfc: RfcHeaders<'x>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub headers_raw: RawHeaders<'x>,
    pub is_encoding_problem: bool,
    pub body: T,
}

impl<'x, T> Part<'x, T> {
    pub fn new(
        headers_rfc: RfcHeaders<'x>,
        headers_raw: RawHeaders<'x>,
        body: T,
        is_encoding_problem: bool,
    ) -> Self {
        Self {
            headers_rfc,
            headers_raw,
            body,
            is_encoding_problem,
        }
    }

    pub fn get_body(&self) -> &T {
        &self.body
    }
}

impl<'x> MultiPart<'x> {
    pub fn new(headers_rfc: RfcHeaders<'x>, headers_raw: RawHeaders<'x>) -> Self {
        Self {
            headers_rfc,
            headers_raw,
        }
    }
}

/// Multipart part
#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct MultiPart<'x> {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub headers_rfc: RfcHeaders<'x>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub headers_raw: RawHeaders<'x>,
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
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum MessagePart<'x> {
    /// Any text/* part
    Text(Part<'x, Cow<'x, str>>),

    /// A text/html part
    Html(Part<'x, Cow<'x, str>>),

    /// Any other part type that is not text.
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    Binary(Part<'x, Cow<'x, [u8]>>),

    /// Any inline binary data that.
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    InlineBinary(Part<'x, Cow<'x, [u8]>>),

    /// Nested RFC5322 message.
    Message(Part<'x, MessageAttachment<'x>>),

    /// Multipart part
    Multipart(MultiPart<'x>),
}

impl<'x> MessagePart<'x> {
    pub fn unwrap_text(&self) -> &Part<'x, Cow<'x, str>> {
        match self {
            MessagePart::Text(part) => part,
            MessagePart::Html(part) => part,
            _ => panic!("Expected text part."),
        }
    }

    pub fn unwrap_binary(&self) -> &Part<'x, Cow<'x, [u8]>> {
        match self {
            MessagePart::Binary(part) => part,
            MessagePart::InlineBinary(part) => part,
            _ => panic!("Expected binary part."),
        }
    }

    pub fn unwrap_message(&self) -> &Message {
        match self {
            MessagePart::Message(part) => match &part.body {
                MessageAttachment::Parsed(message) => message.as_ref(),
                MessageAttachment::Raw(_) => panic!(
                    "This message part has not been parsed yet, use parse_message() instead."
                ),
            },
            _ => panic!("Expected message part."),
        }
    }

    pub fn parse_message(&'x self) -> Option<Message<'x>> {
        match self {
            MessagePart::Message(part) => match &part.body {
                MessageAttachment::Parsed(_) => None,
                MessageAttachment::Raw(raw_message) => Message::parse(raw_message.as_ref()),
            },
            _ => panic!("Expected message part."),
        }
    }
}

/// An RFC5322 or RFC2369 internet address.
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
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

pub type RfcHeaders<'x> = HashMap<RfcHeader, HeaderValue<'x>>;
pub type RawHeaders<'x> = HashMap<HeaderName<'x>, Vec<HeaderOffset>>;

/// Offset of a message element in the raw message.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct HeaderOffset {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum HeaderName<'x> {
    Rfc(RfcHeader),
    Other(Cow<'x, str>),
}

impl<'x> HeaderName<'x> {
    pub fn as_str(&self) -> &str {
        match self {
            HeaderName::Rfc(header) => header.as_str(),
            HeaderName::Other(name) => name.as_ref(),
        }
    }

    pub fn into_owned<'y>(&self) -> HeaderName<'y> {
        match self {
            HeaderName::Rfc(header) => HeaderName::Rfc(*header),
            HeaderName::Other(name) => HeaderName::Other(name.clone().into_owned().into()),
        }
    }

    pub fn unwrap(self) -> String {
        match self {
            HeaderName::Rfc(header) => header.as_str().to_owned(),
            HeaderName::Other(name) => name.into_owned(),
        }
    }
}

/// A header field
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
}

impl RfcHeader {
    pub fn as_str(&self) -> &str {
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
}

impl Display for RfcHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A parsed header value.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
//#[cfg_attr(feature = "serde_support", serde(tag = "type"))]
pub enum HeaderValue<'x> {
    /// A single address
    Address(Addr<'x>),

    /// An address list
    AddressList(Vec<Addr<'x>>),

    /// A group of addresses
    Group(Group<'x>),

    /// A list containing two or more address groups
    GroupList(Vec<Group<'x>>),

    /// A string
    Text(Cow<'x, str>),

    /// A list of strings
    TextList(Vec<Cow<'x, str>>),

    /// A datetime
    DateTime(DateTime),

    /// A Content-Type or Content-Disposition header
    ContentType(ContentType<'x>),

    /// A collection of multiple header fields, for example
    /// Resent-To, References, etc.
    Collection(Vec<HeaderValue<'x>>),
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

    pub fn get_content_type(&self) -> &ContentType<'x> {
        match *self {
            HeaderValue::ContentType(ref ct) => ct,
            _ => panic!("HeaderValue::get_content_type called on non-ContentType value"),
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
            HeaderValue::Collection(list) => {
                HeaderValue::Collection(list.into_iter().map(|v| v.into_owned()).collect())
            }
            HeaderValue::Empty => HeaderValue::Empty,
        }
    }
}

/// An RFC2047 Content-Type or RFC2183 Content-Disposition MIME header field.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ContentType<'x> {
    pub c_type: Cow<'x, str>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub c_subtype: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub attributes: Option<HashMap<Cow<'x, str>, Cow<'x, str>>>,
}

/// An RFC5322 datetime.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DateTime {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub tz_before_gmt: bool,
    pub tz_hour: u32,
    pub tz_minute: u32,
}

impl<'x> Message<'x> {
    /// Returns an iterator over the RFC headers of this message.
    pub fn get_headers_rfc(&self) -> impl Iterator<Item = (&RfcHeader, &HeaderValue<'x>)> {
        self.headers_rfc.iter()
    }

    /// Returns the BCC header field
    pub fn get_bcc(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::Bcc)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the CC header field
    pub fn get_cc(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::Cc)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Comments header fields
    pub fn get_comments(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::Comments)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Date header field
    pub fn get_date(&self) -> Option<&DateTime> {
        self.headers_rfc
            .get(&RfcHeader::Date)
            .and_then(|header| header.as_datetime_ref())
    }

    /// Returns the From header field
    pub fn get_from(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::From)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all In-Reply-To header fields
    pub fn get_in_reply_to(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::InReplyTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Keywords header fields
    pub fn get_keywords(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::Keywords)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Archive header field
    pub fn get_list_archive(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListArchive)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Help header field
    pub fn get_list_help(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListHelp)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-ID header field
    pub fn get_list_id(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListId)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Owner header field
    pub fn get_list_owner(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListOwner)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Post header field
    pub fn get_list_post(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListPost)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Subscribe header field
    pub fn get_list_subscribe(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListSubscribe)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the List-Unsubscribe header field
    pub fn get_list_unsubscribe(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ListUnsubscribe)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Message-ID header field
    pub fn get_message_id(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::MessageId)
            .and_then(|header| header.as_text_ref())
    }

    /// Returns the MIME-Version header field
    pub fn get_mime_version(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::MimeVersion)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Received header fields
    pub fn get_received(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::Received)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all References header fields
    pub fn get_references(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::References)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Reply-To header field
    pub fn get_reply_to(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ReplyTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-BCC header field
    pub fn get_resent_bcc(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentBcc)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-CC header field
    pub fn get_resent_cc(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Resent-Date header fields
    pub fn get_resent_date(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentDate)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-From header field
    pub fn get_resent_from(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentFrom)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Resent-Message-ID header fields
    pub fn get_resent_message_id(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentMessageId)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Sender header field
    pub fn get_resent_sender(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentSender)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Resent-To header field
    pub fn get_resent_to(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ResentTo)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns all Return-Path header fields
    pub fn get_return_path(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ReturnPath)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Sender header field
    pub fn get_sender(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::Sender)
            .unwrap_or(&HeaderValue::Empty)
    }

    /// Returns the Subject header field
    pub fn get_subject(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::Subject)
            .and_then(|header| header.as_text_ref())
    }

    /// Returns the message thread name or 'base subject' as defined in
    /// [RFC 5957 - Internet Message Access Protocol - SORT and THREAD Extensions (Section 2.1)](https://datatracker.ietf.org/doc/html/rfc5256#section-2.1)
    pub fn get_thread_name(&self) -> Option<&str> {
        thread_name(self.get_subject()?).into()
    }

    /// Returns the To header field
    pub fn get_to(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::To)
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

    pub fn _get_raw(&'x self, name: HeaderName) -> Option<Vec<Cow<'x, str>>> {
        self.headers_raw
            .get(&name)?
            .iter()
            .map(|offset| {
                String::from_utf8_lossy(self.raw_message.get(offset.start..offset.end).unwrap())
            })
            .collect::<Vec<Cow<str>>>()
            .into()
    }

    /// Returns in raw format a header field not defined in the conformed RFCs
    pub fn get_other_header(&'x self, name: &str) -> Option<Vec<Cow<'x, str>>> {
        self._get_raw(HeaderName::Other(name.into()))
    }

    /// Returns in raw format a header field defined in the conformed RFCs
    pub fn get_rfc_header(&'x self, name: RfcHeader) -> Option<Vec<Cow<'x, str>>> {
        self._get_raw(HeaderName::Rfc(name))
    }

    fn get_part(&self, list: &'x [MessagePartId], pos: usize) -> Option<&'x dyn BodyPart> {
        match self.parts.get(*list.get(pos)?)? {
            MessagePart::Text(v) => Some(v),
            MessagePart::Html(v) => Some(v),
            MessagePart::Binary(v) => Some(v),
            MessagePart::InlineBinary(v) => Some(v),
            MessagePart::Message(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the transformed contents an inline HTML body part by position
    pub fn get_html_body(&'x self, pos: usize) -> Option<Cow<'x, str>> {
        match self.parts.get(*self.html_body.get(pos)?)? {
            MessagePart::Html(html) => Some(html.body.as_ref().into()),
            MessagePart::Text(text) => Some(text_to_html(text.body.as_ref()).into()),
            _ => None,
        }
    }

    /// Returns the transformed contents an inline text body part by position
    pub fn get_text_body(&'x self, pos: usize) -> Option<Cow<'x, str>> {
        match self.parts.get(*self.text_body.get(pos)?)? {
            MessagePart::Text(text) => Some(text.body.as_ref().into()),
            MessagePart::Html(html) => Some(html_to_text(html.body.as_ref()).into()),
            _ => None,
        }
    }

    /// Returns an inline HTML body part by position
    pub fn get_html_part(&self, pos: usize) -> Option<&Part<Cow<'x, str>>> {
        match self.parts.get(*self.html_body.get(pos)?)? {
            MessagePart::Html(html) => Some(html),
            MessagePart::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Returns an inline text body part by position
    pub fn get_text_part(&self, pos: usize) -> Option<&Part<Cow<'x, str>>> {
        match self.parts.get(*self.text_body.get(pos)?)? {
            MessagePart::Html(html) => Some(html),
            MessagePart::Text(text) => Some(text),
            _ => None,
        }
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
    fn get_content_language(&self) -> &HeaderValue<'x>;
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
        self.headers_rfc
            .get(&RfcHeader::ContentDescription)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_disposition(&self) -> Option<&ContentType> {
        self.headers_rfc
            .get(&RfcHeader::ContentDisposition)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_id(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentId)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_transfer_encoding(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentTransferEncoding)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_type(&self) -> Option<&ContentType> {
        self.headers_rfc
            .get(&RfcHeader::ContentType)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_language(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ContentLanguage)
            .unwrap_or(&HeaderValue::Empty)
    }

    fn get_content_location(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentLocation)
            .and_then(|header| header.as_text_ref())
    }
}

/// An inline Text or Binary body part.
pub trait BodyPart<'x>: fmt::Display + MimeHeaders<'x> {
    /// Returns the body part's contents as a `u8` slice
    fn get_contents(&'x self) -> &'x [u8];

    /// Returns the body part's contents as a `str`
    fn get_text_contents(&'x self) -> &'x str;

    /// Returns the body part's length
    fn len(&self) -> usize;

    /// Returns `true` when the body part MIME type is text/*
    fn is_text(&self) -> bool;

    /// Returns `true` when the part is not text
    fn is_binary(&self) -> bool;

    /// Returns `true` when the body part is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'x> BodyPart<'x> for Part<'x, Cow<'x, str>> {
    fn get_contents(&'x self) -> &'x [u8] {
        self.body.as_bytes()
    }

    fn is_text(&self) -> bool {
        true
    }

    fn is_binary(&self) -> bool {
        false
    }

    fn get_text_contents(&'x self) -> &'x str {
        self.body.as_ref()
    }

    fn len(&self) -> usize {
        self.body.len()
    }
}

impl<'x> fmt::Display for Part<'x, Cow<'x, str>> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.body.as_ref())
    }
}

impl<'x> BodyPart<'x> for Part<'x, Cow<'x, [u8]>> {
    fn get_contents(&'x self) -> &'x [u8] {
        self.body.as_ref()
    }

    fn get_text_contents(&'x self) -> &'x str {
        std::str::from_utf8(self.body.as_ref()).unwrap_or("")
    }

    fn is_text(&self) -> bool {
        false
    }

    fn is_binary(&self) -> bool {
        true
    }

    fn len(&self) -> usize {
        self.body.len()
    }
}

impl<'x> fmt::Display for Part<'x, Cow<'x, [u8]>> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("[binary contents]")
    }
}

impl<'x> BodyPart<'x> for Part<'x, MessageAttachment<'x>> {
    fn get_contents(&'x self) -> &'x [u8] {
        match &self.body {
            MessageAttachment::Parsed(msg) => msg.raw_message.as_ref(),
            MessageAttachment::Raw(raw) => raw.as_ref(),
        }
    }

    fn get_text_contents(&'x self) -> &'x str {
        std::str::from_utf8(self.get_contents()).unwrap_or("")
    }

    fn is_text(&self) -> bool {
        false
    }

    fn is_binary(&self) -> bool {
        true
    }

    fn len(&self) -> usize {
        match &self.body {
            MessageAttachment::Parsed(msg) => msg.raw_message.len(),
            MessageAttachment::Raw(raw) => raw.len(),
        }
    }
}

impl<'x> fmt::Display for Part<'x, MessageAttachment<'x>> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.get_text_contents())
    }
}

impl<'x, T> MimeHeaders<'x> for Part<'x, T> {
    fn get_content_description(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentDescription)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_disposition(&self) -> Option<&ContentType> {
        self.headers_rfc
            .get(&RfcHeader::ContentDisposition)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_id(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentId)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_transfer_encoding(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentTransferEncoding)
            .and_then(|header| header.as_text_ref())
    }

    fn get_content_type(&self) -> Option<&ContentType> {
        self.headers_rfc
            .get(&RfcHeader::ContentType)
            .and_then(|header| header.as_content_type_ref())
    }

    fn get_content_language(&self) -> &HeaderValue<'x> {
        self.headers_rfc
            .get(&RfcHeader::ContentLanguage)
            .unwrap_or(&HeaderValue::Empty)
    }

    fn get_content_location(&self) -> Option<&str> {
        self.headers_rfc
            .get(&RfcHeader::ContentLocation)
            .and_then(|header| header.as_text_ref())
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
    type Item = &'x dyn BodyPart<'x>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.message.get_part(self.list, self.pos as usize)
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
    pub fn get_type(&'x self) -> &'x str {
        &self.c_type
    }

    /// Returns the sub-type
    pub fn get_subtype(&'x self) -> Option<&'x str> {
        self.c_subtype.as_ref()?.as_ref().into()
    }

    /// Returns an attribute by name
    pub fn get_attribute(&'x self, name: &str) -> Option<&'x str> {
        self.attributes.as_ref()?.get(name)?.as_ref().into()
    }

    /// Returns `true` when the provided attribute name is present
    pub fn has_attribute(&'x self, name: &str) -> bool {
        self.attributes
            .as_ref()
            .map_or_else(|| false, |attr| attr.contains_key(name))
    }

    /// Returns ```true``` if the Content-Disposition type is "attachment"
    pub fn is_attachment(&'x self) -> bool {
        self.c_type.eq_ignore_ascii_case("attachment")
    }

    /// Returns ```true``` if the Content-Disposition type is "inline"
    pub fn is_inline(&'x self) -> bool {
        self.c_type.eq_ignore_ascii_case("inline")
    }
}

/// Contents of an e-mail message attachment.
#[derive(Debug, PartialEq)]
pub enum MessageAttachment<'x> {
    Parsed(Box<Message<'x>>),
    Raw(Cow<'x, [u8]>),
}

impl<'x> Default for MessageAttachment<'x> {
    fn default() -> Self {
        MessageAttachment::Raw((&[] as &[u8]).into())
    }
}

impl<'x> MessageAttachment<'x> {
    /// Parse the message attachment and return a `Message` object.
    pub fn parse_raw(&'x self) -> Option<Message<'x>> {
        if let MessageAttachment::Raw(raw) = self {
            Message::parse(raw.as_ref())
        } else {
            None
        }
    }

    /// Returns a reference to the parsed `Message` object.
    pub fn as_ref(&self) -> Option<&Message> {
        if let MessageAttachment::Parsed(message) = self {
            message.as_ref().into()
        } else {
            None
        }
    }

    /// Returns ```true``` is the message has been parsed.
    /// Call `MessageAttachment::parse_raw` when this method returns ```false```,
    /// otherwise use `MessageAttachment::as_ref`.
    pub fn is_parsed(&self) -> bool {
        matches!(self, MessageAttachment::Parsed(_))
    }

    /*/// Returns the parsed message if available, otherwise
    /// parses the raw message and returns the results.
    pub fn unwrap(&'x mut self) -> Option<Message<'x>> {
        match self {
            MessageAttachment::Parsed(message) => (*std::mem::take(&mut message).unwrap()).into(),
            MessageAttachment::Raw(raw) => Message::parse(raw.as_ref()),
        }
    }*/
}

impl<'x> Part<'x, MessageAttachment<'x>> {
    /// Parse the message attachment and return a `Message` object.
    pub fn parse_raw(&'x self) -> Option<Message<'x>> {
        self.body.parse_raw()
    }

    /// Returns a reference to the parsed `Message` object.
    pub fn as_ref(&self) -> Option<&Message> {
        self.body.as_ref()
    }

    /// Returns ```true``` is the message has been parsed.
    /// Call `MessageAttachment::parse_raw` when this method returns ```false```,
    /// otherwise use `MessageAttachment::as_ref`.
    pub fn is_parsed(&self) -> bool {
        self.body.is_parsed()
    }
}

#[cfg(feature = "serde_support")]
impl<'x> Serialize for MessageAttachment<'x> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MessageAttachment::Parsed(message) => message.serialize(serializer),
            MessageAttachment::Raw(raw) => Message::parse(raw.as_ref())
                .ok_or_else(|| serde::ser::Error::custom("Failed to parse message attachment."))?
                .serialize(serializer),
        }
    }
}

#[cfg(feature = "serde_support")]
impl<'x, 'de> Deserialize<'de> for MessageAttachment<'x> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        panic!("Deserializing message attachments is not supported at this time.")
    }
}
