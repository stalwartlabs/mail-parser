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
//! Multipurpose Internet Mail Extensions (MIME; _RFC 2045 - 2049_) as well as other [internet messaging RFCs](#conformed-rfcs).
//! 
//! It also supports decoding messages in [41 different character sets](#supported-character-sets) including obsolete formats such as UTF-7.
//! All Unicode (UTF-*) and single-byte character sets are handled internally by the library while support for legacy multi-byte encodings of Chinese
//! and Japanse languages such as BIG5 or ISO-2022-JP is provided by the optional dependency [encoding_rs](https://crates.io/crates/encoding_rs).
//! 
//! In general, this library abides by the Postel's law or [Robustness Principle](https://en.wikipedia.org/wiki/Robustness_principle) which 
//! states that an implementation must be conservative in its sending behavior and liberal in its receiving behavior. This means that
//! _mail-parser_ will make a best effort to parse non-conformat e-mail messages as long as these do not deviate too much from the standard.
//! 
//! Unlike other e-mail parsing libraries that return nested representations of the different MIME parts in a message, this library 
//! conforms to [RFC 8621, Section 4.1.4](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4) and provides a more human-friendly
//! representation of the message contents consisting of just text body parts, html body parts and attachments. Additionally, conversion to/from
//! HTML and plain text inline body parts is done automatically when the _alternative_ version is missing.
//! 
//! Performance and memory safety were two important factors while designing _mail-parser_:
//! 
//! - **Zero-copy parsing** is done in most cases (unless when decoding non-UTF8 text or when RFC2047/RFC2231 encoded parts are present). 
//!   Practically all strings and u8 slices returned by this library are `Cow<str>` or `Cow<[u8]>` references to the input raw message.
//! - Memory allocations are always avoided unless they are really necessary. In fact, all Base64 and Quoted-Printable parts are decoded in 
//!   place re-using the input buffer. 
//! - [Perfect hashing](https://en.wikipedia.org/wiki/Perfect_hash_function) is used for fast look-up of message header fields, character 
//!   set names and aliases, HTML entities as well as month names while parsing _Date_ fields.
//! - Although some `unsafe` code was used to obtain performance gains of about 10%, every function in the library has been 
//!   [fuzzed](#testing-fuzzing--benchmarking) and also heavily [tested with MIRI](#testing-fuzzing--benchmarking).
//! - Fully battle-tested with millions of real-world e-mail messages dating from 1995 until today.
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
//! - [RFC 2183 - Communicating Presentation Information in Internet Messages: The Content-Disposition Header Field](https://datatracker.ietf.org/doc/html/rfc2183)
//! - [RFC 6532 - Internationalized Email Headers](https://datatracker.ietf.org/doc/html/rfc6532)
//! - [RFC 2152 - UTF-7 - A Mail-Safe Transformation Format of Unicode](https://datatracker.ietf.org/doc/html/rfc2152)
//! - [RFC 2369 - The Use of URLs as Meta-Syntax for Core Mail List Commands and their Transport through Message Header Fields](https://datatracker.ietf.org/doc/html/rfc2369)
//! - [RFC 2919 - List-Id: A Structured Field and Namespace for the Identification of Mailing Lists](https://datatracker.ietf.org/doc/html/rfc2919)
//! - [RFC 8621 - The JSON Meta Application Protocol (JMAP) for Mail (Section 4.1.4)](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4)
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
//!    let mut input = concat!(
//!        "From: Art Vandelay <art@vandelay.com> (Vandelay Industries)\n",
//!        "To: \"Colleagues\": \"James Smythe\" <james@vandelay.com>; Friends:\n",
//!        "    jane@example.com, =?UTF-8?Q?John_Sm=C3=AEth?= <john@example.com>;\n",
//!        "Date: Sat, 20 Nov 2021 14:22:01 -0800\n",
//!        "Subject: Why not both importing AND exporting? =?utf-8?b?4pi6?=\n",
//!        "Content-Type: multipart/mixed; boundary=\"festivus\";\n\n",
//!        "--festivus\n",
//!        "Content-Type: text/html; charset=\"us-ascii\"\n",
//!        "Content-Transfer-Encoding: base64\n\n",
//!        "PGh0bWw+PHA+SSB3YXMgdGhpbmtpbmcgYWJvdXQgcXVpdHRpbmcgdGhlICZsZHF1bztle\n",
//!        "HBvcnRpbmcmcmRxdW87IHRvIGZvY3VzIGp1c3Qgb24gdGhlICZsZHF1bztpbXBvcnRpbm\n",
//!        "cmcmRxdW87LDwvcD48cD5idXQgdGhlbiBJIHRob3VnaHQsIHdoeSBub3QgZG8gYm90aD8\n",
//!        "gJiN4MjYzQTs8L3A+PC9odG1sPg==\n",
//!        "--festivus\n",
//!        "Content-Type: message/rfc822\n\n",
//!        "From: \"Cosmo Kramer\" <kramer@kramerica.com>\n",
//!        "Subject: Exporting my book about coffee tables\n",
//!        "Content-Type: multipart/mixed; boundary=\"giddyup\";\n\n",
//!        "--giddyup\n",
//!        "Content-Type: text/plain; charset=\"utf-16\"\n",
//!        "Content-Transfer-Encoding: quoted-printable\n\n",
//!        "=FF=FE=0C!5=D8\"=DD5=D8)=DD5=D8-=DD =005=D8*=DD5=D8\"=DD =005=D8\"=\n",
//!        "=DD5=D85=DD5=D8-=DD5=D8,=DD5=D8/=DD5=D81=DD =005=D8*=DD5=D86=DD =\n",
//!        "=005=D8=1F=DD5=D8,=DD5=D8,=DD5=D8(=DD =005=D8-=DD5=D8)=DD5=D8\"=\n",
//!        "=DD5=D8=1E=DD5=D80=DD5=D8\"=DD!=00\n",
//!        "--giddyup\n",
//!        "Content-Type: image/gif; name*1=\"about \"; name*0=\"Book \";\n",
//!        "              name*2*=utf-8''%e2%98%95 tables.gif\n",
//!        "Content-Transfer-Encoding: Base64\n",
//!        "Content-Disposition: attachment\n\n",
//!        "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7\n",
//!        "--giddyup--\n",
//!        "--festivus--\n",
//!    )
//!    .as_bytes()
//!    .to_vec();
//!
//!    let message = Message::parse(&mut input[..]);
//!
//!    // Parses addresses (including comments), lists and groups
//!    assert_eq!(
//!        message.get_from(),
//!        &Address::Address(Addr {
//!            name: Some("Art Vandelay (Vandelay Industries)".into()),
//!            address: Some("art@vandelay.com".into())
//!        })
//!    );
//!    assert_eq!(
//!        message.get_to(),
//!        &Address::GroupList(vec![
//!            Group {
//!                name: Some("Colleagues".into()),
//!                addresses: vec![Addr {
//!                    name: Some("James Smythe".into()),
//!                    address: Some("james@vandelay.com".into())
//!                }]
//!            },
//!            Group {
//!                name: Some("Friends".into()),
//!                addresses: vec![
//!                    Addr {
//!                        name: None,
//!                        address: Some("jane@example.com".into())
//!                    },
//!                    Addr {
//!                        name: Some("John Smîth".into()),
//!                        address: Some("john@example.com".into())
//!                    }
//!                ]
//!            }
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
//!        message.get_html_body(0).unwrap().to_string(),
//!        concat!(
//!            "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
//!            "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
//!            " why not do both? &#x263A;</p></html>"
//!        )
//!    );
//!
//!    // HTML parts are converted to plain text (and viceversa) when missing
//!    assert_eq!(
//!        message.get_text_body(0).unwrap().to_string(),
//!        concat!(
//!            "I was thinking about quitting the “exporting” to focus just on the",
//!            " “importing”,\nbut then I thought, why not do both? ☺\n"
//!        )
//!    );
//!
//!    // Supports nested messages as well as multipart/digest
//!    let nested_message = match message.get_attachment(0).unwrap() {
//!        MessagePart::Message(v) => v,
//!        _ => unreachable!(),
//!    };
//!
//!    assert_eq!(
//!        nested_message.get_subject().unwrap(),
//!        "Exporting my book about coffee tables"
//!    );
//!
//!    // Handles UTF-* as well as many legacy encodings
//!    assert_eq!(
//!        nested_message.get_text_body(0).unwrap().to_string(),
//!        "ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!"
//!    );
//!    assert_eq!(
//!        nested_message.get_html_body(0).unwrap().to_string(),
//!        "<html><body>ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!</body></html>"
//!    );
//!
//!    let nested_attachment = match nested_message.get_attachment(0).unwrap() {
//!        MessagePart::Binary(v) => v,
//!        _ => unreachable!(),
//!    };
//!
//!    assert_eq!(nested_attachment.len(), 42);
//!
//!    // Full RFC2231 support for continuations and character sets
//!    assert_eq!(
//!        nested_attachment
//!            .get_header()
//!            .unwrap()
//!            .get_content_type()
//!            .unwrap()
//!            .get_attribute("name")
//!            .unwrap(),
//!        "Book about ☕ tables.gif"
//!    );
//!
//!    // Integrates with Serde
//!    println!("{}", serde_json::to_string_pretty(&message).unwrap());
//!    println!("{}", serde_yaml::to_string(&message).unwrap());
//!```


pub mod decoders;
pub mod parsers;

use std::{borrow::Cow, collections::HashMap, fmt, slice::Iter};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// An RFC5322/RFC822 message.
#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Message<'x> {
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    header: Box<MessageHeader<'x>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Vec::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    html_body: Vec<InlinePart<'x>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Vec::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    text_body: Vec<InlinePart<'x>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Vec::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    attachments: Vec<MessagePart<'x>>,
}

/// A text message part.
#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct TextPart<'x> {
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    header: Option<MimeHeader<'x>>,
    contents: Cow<'x, str>,
}

/// A binary (`[u8]`) message part.
#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct BinaryPart<'x> {
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    header: Option<MimeHeader<'x>>,
    #[cfg_attr(feature = "serde_support", serde(with = "serde_bytes"))]
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    contents: Cow<'x, [u8]>,
}

#[doc(hidden)]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(untagged))]
pub enum InlinePart<'x> {
    Text(TextPart<'x>),
    InlineBinary(u32),
}

/// A text, binary or nested e-mail MIME message part.
/// 
/// - Text: Any text/* part
/// - Binary: Any other part type that is not text, usually attachments.
/// - InlineBinary: Same as the Binary variant but an inline part according to RFC 8621, Section 4.1.4
/// - Message: A nested RFC5322 message.
/// 
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum MessagePart<'x> {
    /// Any text/* part
    Text(TextPart<'x>),

    /// Any other part type that is not text, usually attachments.
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    Binary(BinaryPart<'x>),

    /// Same as the Binary variant but an inline part according to RFC 8621, Section 4.1.4
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    InlineBinary(BinaryPart<'x>),

    /// A nested RFC5322 message.
    Message(Message<'x>),
}

/// An RFC5322 message header.
#[derive(PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct MessageHeader<'x> {
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bcc: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cc: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub comments: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub date: Option<DateTime>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub from: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub in_reply_to: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub keywords: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_archive: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_help: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_id: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_owner: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_post: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_subscribe: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub list_unsubscribe: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub message_id: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub mime_version: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub received: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub references: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub reply_to: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_bcc: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_cc: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_date: Option<Vec<DateTime>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_from: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_message_id: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_sender: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resent_to: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub return_path: Option<Vec<Cow<'x, str>>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub sender: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub subject: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Address::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub to: Address<'x>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_description: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_disposition: Option<ContentType<'x>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_id: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_transfer_encoding: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_type: Option<ContentType<'x>>,
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "HashMap::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub others: HashMap<&'x str, Vec<Cow<'x, str>>>,
}

/// A MIME header.
#[derive(PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct MimeHeader<'x> {
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_description: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_disposition: Option<ContentType<'x>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_id: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_transfer_encoding: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub content_type: Option<ContentType<'x>>,
}

/// An RFC5322 or RFC2369 internet address.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Addr<'x> {
    /// The address name including comments
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name: Option<Cow<'x, str>>,

    /// An e-mail address (RFC5322/RFC2369) or URL (RFC2369)
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub address: Option<Cow<'x, str>>,
}

/// An RFC5322 address group.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Group<'x> {
    /// Group name
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name: Option<Cow<'x, str>>,

    /// Addressess member of the group
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Vec::is_empty"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub addresses: Vec<Addr<'x>>,
}

/// An RFC5322 or RFC2369 address field.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Address<'x> {
    /// A single address
    Address(Addr<'x>),

    /// An address list
    AddressList(Vec<Addr<'x>>),

    /// A group of addresses
    Group(Group<'x>),

    /// A list containing two or more groups
    GroupList(Vec<Group<'x>>),

    /// A collection of address fields, used for header fields
    /// that might be present more than once in a message,
    /// for example Resent-To
    Collection(Vec<Address<'x>>),
    Empty,
}

/// An RFC2047 Content-Type or RFC2183 Content-Disposition MIME header field.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ContentType<'x> {
    c_type: Cow<'x, str>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    c_subtype: Option<Cow<'x, str>>,
    #[cfg_attr(feature = "serde_support", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    attributes: Option<HashMap<Cow<'x, str>, Cow<'x, str>>>,
}

/// An RFC5322 datetime.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DateTime {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    tz_before_gmt: bool,
    tz_hour: u32,
    tz_minute: u32,
}

/// MIME header fields.
pub trait MimeFieldGet<'x> {
    /// Content-Description MIME header
    fn get_content_description(&self) -> Option<&Cow<'x, str>>;
    /// Content-Disposition MIME header
    fn get_content_disposition(&self) -> Option<&ContentType<'x>>;
    /// Content-ID MIME header
    fn get_content_id(&self) -> Option<&Cow<'x, str>>;
    /// Content-Transfer-Encoding MIME header
    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>>;
    /// Content-Type MIME header
    fn get_content_type(&self) -> Option<&ContentType<'x>>;
}

impl<'x> Message<'x> {
    /// Returns the BCC header field
    pub fn get_bcc(&self) -> &Address<'x> {
        &self.header.bcc
    }

    /// Returns the CC header field
    pub fn get_cc(&self) -> &Address<'x> {
        &self.header.cc
    }

    /// Returns all Comments header fields
    pub fn get_comments(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.comments.as_ref()
    }

    /// Returns the Date header field
    pub fn get_date(&self) -> Option<&DateTime> {
        self.header.date.as_ref()
    }

    /// Returns the From header field
    pub fn get_from(&self) -> &Address<'x> {
        &self.header.from
    }

    /// Returns all In-Reply-To header fields
    pub fn get_in_reply_to(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.in_reply_to.as_ref()
    }

    /// Returns all Keywords header fields
    pub fn get_keywords(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.keywords.as_ref()
    }

    /// Returns the List-Archive header field
    pub fn get_list_archive(&self) -> &Address<'x> {
        &self.header.list_archive
    }

    /// Returns the List-Help header field
    pub fn get_list_help(&self) -> &Address<'x> {
        &self.header.list_help
    }

    /// Returns the List-ID header field
    pub fn get_list_id(&self) -> &Address<'x> {
        &self.header.list_id
    }

    /// Returns the List-Owner header field
    pub fn get_list_owner(&self) -> &Address<'x> {
        &self.header.list_owner
    }

    /// Returns the List-Port header field
    pub fn get_list_post(&self) -> &Address<'x> {
        &self.header.list_post
    }

    /// Returns the List-Subscribe header field
    pub fn get_list_subscribe(&self) -> &Address<'x> {
        &self.header.list_subscribe
    }

    /// Returns the List-Unsubscribe header field
    pub fn get_list_unsubscribe(&self) -> &Address<'x> {
        &self.header.list_unsubscribe
    }

    /// Returns the Message-ID header field
    pub fn get_message_id(&self) -> Option<&Cow<'x, str>> {
        self.header.message_id.as_ref()
    }

    /// Returns the MIME-Version header field
    pub fn get_mime_version(&self) -> Option<&Cow<'x, str>> {
        self.header.mime_version.as_ref()
    }

    /// Returns all Received header fields
    pub fn get_received(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.received.as_ref()
    }

    /// Returns all References header fields
    pub fn get_references(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.references.as_ref()
    }

    /// Returns the Reply-To header field
    pub fn get_reply_to(&self) -> &Address<'x> {
        &self.header.reply_to
    }

    /// Returns the Resent-BCC header field
    pub fn get_resent_bcc(&self) -> &Address<'x> {
        &self.header.bcc
    }

    /// Returns the Resent-CC header field
    pub fn get_resent_cc(&self) -> &Address<'x> {
        &self.header.resent_to
    }

    /// Returns all Resent-Date header fields
    pub fn get_resent_date(&self) -> Option<&Vec<DateTime>> {
        self.header.resent_date.as_ref()
    }

    /// Returns the Resent-From header field
    pub fn get_resent_from(&self) -> &Address<'x> {
        &self.header.resent_from
    }

    /// Returns all Resent-Message-ID header fields
    pub fn get_resent_message_id(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.resent_message_id.as_ref()
    }

    /// Returns the Sender header field
    pub fn get_resent_sender(&self) -> &Address<'x> {
        &self.header.resent_sender
    }

    /// Returns the Resent-To header field
    pub fn get_resent_to(&self) -> &Address<'x> {
        &self.header.resent_to
    }

    /// Returns all Return-Path header fields
    pub fn get_return_path(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.return_path.as_ref()
    }

    /// Returns the Sender header field
    pub fn get_sender(&self) -> &Address<'x> {
        &self.header.sender
    }

    /// Returns the Subject header field
    pub fn get_subject(&self) -> Option<&Cow<'x, str>> {
        self.header.subject.as_ref()
    }

    /// Returns the To header field
    pub fn get_to(&self) -> &Address<'x> {
        &self.header.to
    }

    /// Returns any other message header not specified in RFCs 5322, 2045-2049, 2183, 2369 or 2919.
    pub fn get_header(&self, name: &str) -> Option<&Vec<Cow<'x, str>>> {
        self.header.others.get(name)
    }

    fn get_body_part(&self, list: &'x [InlinePart<'x>], pos: usize) -> Option<&'x dyn BodyPart> {
        match list.get(pos) {
            Some(InlinePart::Text(v)) => Some(v),
            Some(InlinePart::InlineBinary(v)) => match self.attachments.get(*v as usize)? {
                MessagePart::Text(v) => Some(v),
                MessagePart::Binary(v) | MessagePart::InlineBinary(v) => Some(v),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns an inline HTML body part by position
    pub fn get_html_body(&self, pos: usize) -> Option<&dyn BodyPart> {
        self.get_body_part(&self.html_body, pos)
    }

    /// Returns an inline text body part by position
    pub fn get_text_body(&self, pos: usize) -> Option<&dyn BodyPart> {
        self.get_body_part(&self.text_body, pos)
    }

    /// Returns an attacment by position
    pub fn get_attachment(&self, pos: usize) -> Option<&MessagePart<'x>> {
        self.attachments.get(pos)
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
    pub fn get_attachments(&self) -> Iter<MessagePart<'_>> {
        self.attachments.iter()
    }
}

impl<'x> MimeFieldGet<'x> for Message<'x> {
    fn get_content_description(&self) -> Option<&Cow<'x, str>> {
        self.header.content_description.as_ref()
    }

    fn get_content_disposition(&self) -> Option<&ContentType<'x>> {
        self.header.content_disposition.as_ref()
    }

    fn get_content_id(&self) -> Option<&Cow<'x, str>> {
        self.header.content_id.as_ref()
    }

    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>> {
        self.header.content_transfer_encoding.as_ref()
    }

    fn get_content_type(&self) -> Option<&ContentType<'x>> {
        self.header.content_type.as_ref()
    }
}

impl<'x> MimeFieldGet<'x> for MimeHeader<'x> {
    fn get_content_description(&self) -> Option<&Cow<'x, str>> {
        self.content_description.as_ref()
    }

    fn get_content_disposition(&self) -> Option<&ContentType<'x>> {
        self.content_disposition.as_ref()
    }

    fn get_content_id(&self) -> Option<&Cow<'x, str>> {
        self.content_id.as_ref()
    }

    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>> {
        self.content_transfer_encoding.as_ref()
    }

    fn get_content_type(&self) -> Option<&ContentType<'x>> {
        self.content_type.as_ref()
    }
}

impl<'x> MimeFieldGet<'x> for MessageHeader<'x> {
    fn get_content_description(&self) -> Option<&Cow<'x, str>> {
        self.content_description.as_ref()
    }

    fn get_content_disposition(&self) -> Option<&ContentType<'x>> {
        self.content_disposition.as_ref()
    }

    fn get_content_id(&self) -> Option<&Cow<'x, str>> {
        self.content_id.as_ref()
    }

    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>> {
        self.content_transfer_encoding.as_ref()
    }

    fn get_content_type(&self) -> Option<&ContentType<'x>> {
        self.content_type.as_ref()
    }
}

/// An inline Text or Binary body part.
pub trait BodyPart<'x>: fmt::Display {
    /// Returns the MIME Header of the part
    fn get_header(&self) -> Option<&MimeHeader<'x>>;

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

impl<'x> BodyPart<'x> for TextPart<'x> {
    fn get_header(&self) -> Option<&MimeHeader<'x>> {
        self.header.as_ref()
    }

    fn get_contents(&'x self) -> &'x [u8] {
        self.contents.as_bytes()
    }

    fn is_text(&self) -> bool {
        true
    }

    fn is_binary(&self) -> bool {
        false
    }

    fn get_text_contents(&'x self) -> &'x str {
        self.contents.as_ref()
    }

    fn len(&self) -> usize {
        self.contents.len()
    }
}

impl<'x> fmt::Display for TextPart<'x> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.contents.as_ref())
    }
}

impl<'x> BodyPart<'x> for BinaryPart<'x> {
    fn get_header(&self) -> Option<&MimeHeader<'x>> {
        self.header.as_ref()
    }

    fn get_contents(&'x self) -> &'x [u8] {
        self.contents.as_ref()
    }

    fn get_text_contents(&'x self) -> &'x str {
        std::str::from_utf8(self.contents.as_ref()).unwrap_or("")
    }

    fn is_text(&self) -> bool {
        false
    }

    fn is_binary(&self) -> bool {
        true
    }

    fn len(&self) -> usize {
        self.contents.len()
    }
}

impl<'x> fmt::Display for BinaryPart<'x> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("[binary contents]")
    }
}

#[doc(hidden)]
pub struct BodyPartIterator<'x> {
    message: &'x Message<'x>,
    list: &'x [InlinePart<'x>],
    pos: isize,
}

impl<'x> BodyPartIterator<'x> {
    fn new(message: &'x Message<'x>, list: &'x [InlinePart<'x>]) -> BodyPartIterator<'x> {
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
        self.message.get_body_part(self.list, self.pos as usize)
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
