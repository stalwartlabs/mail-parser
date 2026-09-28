/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Fast and robust e-mail parsing.
//!
//! A [`MessageParser`] turns the bytes of an RFC 5322 message into a
//! read-only [`Message`]: the raw bytes plus a flat, index-based store of
//! every nested message, MIME part and header field. Everything is read
//! through small `Copy` views ([`MessageRef`], [`MessagePart`], [`Header`],
//! [`HeaderValue`], ...), and bodies are decoded on access.
//!
//! ```
//! use mail_parser::MessageParser;
//!
//! let raw = b"From: Ann <ann@example.com>\r\nSubject: Hi\r\n\r\nHello!\r\n";
//! let message = MessageParser::new().parse(raw).expect("message");
//! assert_eq!(message.subject(), Some("Hi"));
//! assert_eq!(message.body_text(0).as_deref(), Some("Hello!\r\n"));
//! ```
//!
//! - Parsing: [`MessageParser::parse`], [`MessageParser::parse_owned`],
//!   [`MessageParser::parse_headers`], and [`MessageParser::parse_with`]
//!   with [`MessageBuffers`] to reuse storage from one message to the next.
//! - Structure: [`Message::parts`] and [`Message::messages`] in document
//!   order, [`MessagePart::children`], [`MessagePart::nested`], the RFC 8621
//!   body lists ([`MessageRef::text_body`], [`MessageRef::html_body`],
//!   [`MessageRef::attachments`]) and MIME boundaries
//!   ([`MessagePart::boundary`], [`Message::part_by_boundary`]).
//! - Bodies: [`MessagePart::decoded`], [`MessagePart::text`],
//!   [`MessagePart::text_prefix`] and [`MessagePart::decoded_len`];
//!   [`MessagePart::decoded_checked`] and [`MessagePart::text_checked`]
//!   also report decoding problems.
//! - Header fields: [`Headers`] with typed getters, [`Header::raw_name`]
//!   and [`Header::raw_value`] as written, [`Header::parse_as`] to parse a
//!   field in another form, and [`HeaderForm::parse`] for values on their
//!   own.
//! - Mailboxes: [`mailbox::mbox`] and [`mailbox::maildir`].
//! - Utilities: [`DateTime`], [`thread_name`], [`html_to_text`],
//!   [`text_to_html`], [`preview_text`], [`Charset`] and the other
//!   [`decoders`].
//!
//! Parsing never fails on malformed input: damaged structure is reported
//! by [`MessagePart::flags`], malformed encodings by the checked decoding
//! methods. The `full_encoding` feature adds the multi-byte charsets
//! (through `encoding_rs`), and the `serde` feature serializes messages.

#![warn(missing_docs)]

mod datetime;
pub mod decoders;
mod fields;
mod header_name;
pub mod mailbox;
mod parser;
#[doc(hidden)]
pub mod scan;
mod store;
mod view;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

pub use datetime::{DOW, DateTime, MONTH};
pub use decoders::{
    charsets::Charset,
    html::{add_html_token, html_to_text, strip_charset_meta, text_to_html},
    preview::{preview_html, preview_text, truncate_html, truncate_text},
};
pub use fields::{
    HeaderForm, ParsedValue,
    address::{
        parse_address_detail_part, parse_address_domain, parse_address_local_part,
        parse_address_user_part,
    },
    thread::thread_name,
};
pub use header_name::{HeaderKey, HeaderName};
pub use parser::MessageParser;
pub use store::{MessageBuffers, MessageId, PartId};
pub use view::{
    Address, AddressList, AddressRun, ContentType, DecodeProblems, Greeting, Group, Header,
    HeaderIter, HeaderValue, Headers, Host, Mailbox, Message, MessagePart, MessageRef,
    NamedHeaders, PartFlags, PartKind, PartRole, Protocol, Received, Source, TextList, TlsVersion,
};

/// Declared Content-Transfer-Encoding of a part. The numeric values (0, 1,
/// 2) are stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Encoding {
    /// No transfer encoding: `7bit`, `8bit`, `binary`, no field, or a value
    /// the parser does not know.
    #[default]
    None = 0,
    /// `quoted-printable`.
    QuotedPrintable = 1,
    /// `base64`.
    Base64 = 2,
}

impl From<u8> for Encoding {
    fn from(value: u8) -> Self {
        match value {
            1 => Encoding::QuotedPrintable,
            2 => Encoding::Base64,
            _ => Encoding::None,
        }
    }
}

impl From<Encoding> for u8 {
    fn from(value: Encoding) -> Self {
        value as u8
    }
}
