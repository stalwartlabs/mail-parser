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

use core::fmt;
use std::hash::Hash;
use std::{borrow::Cow, fmt::Display};

use crate::{
    ContentType, DateTime, GetHeader, Header, HeaderName, HeaderValue, Host, Message, MessagePart,
    MessagePartId, MimeHeaders, PartType, Received, RfcHeader,
};

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
    pub fn into_owned(self) -> Header<'static> {
        Header {
            name: self.name.into_owned(),
            value: self.value.into_owned(),
            offset_field: self.offset_field,
            offset_start: self.offset_start,
            offset_end: self.offset_end,
        }
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

    pub fn unwrap_received(self) -> Received<'x> {
        match self {
            HeaderValue::Received(r) => *r,
            _ => panic!("HeaderValue::unwrap_received called on non-Received value"),
        }
    }

    pub fn as_text(&self) -> Option<&str> {
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

    pub fn as_received(&self) -> Option<&Received> {
        match *self {
            HeaderValue::Received(ref r) => Some(r),
            _ => None,
        }
    }

    pub fn content_type(&self) -> &ContentType<'x> {
        match *self {
            HeaderValue::ContentType(ref ct) => ct,
            _ => panic!(
                "HeaderValue::content_type called on non-ContentType: {:?}",
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

    pub fn into_owned(self) -> HeaderValue<'static> {
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
            HeaderValue::Received(rcvd) => HeaderValue::Received(Box::new(rcvd.into_owned())),
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
            HeaderValue::Received(_) => 1,
            HeaderValue::Empty => 0,
        }
    }
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
    pub fn as_str(&self) -> &str {
        match self {
            HeaderName::Rfc(header) => header.as_str(),
            HeaderName::Other(name) => name.as_ref(),
        }
    }

    pub fn to_owned(&self) -> HeaderName<'static> {
        match self {
            HeaderName::Rfc(header) => HeaderName::Rfc(*header),
            HeaderName::Other(name) => HeaderName::Other(name.clone().into_owned().into()),
        }
    }

    pub fn into_owned(self) -> HeaderName<'static> {
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

impl<'x> MimeHeaders<'x> for Message<'x> {
    fn content_description(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentDescription)
            .and_then(|header| header.as_text())
    }

    fn content_disposition(&self) -> Option<&ContentType> {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentDisposition)
            .and_then(|header| header.as_content_type_ref())
    }

    fn content_id(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentId)
            .and_then(|header| header.as_text())
    }

    fn content_transfer_encoding(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentTransferEncoding)
            .and_then(|header| header.as_text())
    }

    fn content_type(&self) -> Option<&ContentType> {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentType)
            .and_then(|header| header.as_content_type_ref())
    }

    fn content_language(&self) -> &HeaderValue {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentLanguage)
            .unwrap_or(&HeaderValue::Empty)
    }

    fn content_location(&self) -> Option<&str> {
        self.parts[0]
            .headers
            .rfc(&RfcHeader::ContentLocation)
            .and_then(|header| header.as_text())
    }
}

impl<'x> MessagePart<'x> {
    /// Returns the body part's contents as a `u8` slice
    pub fn contents(&'x self) -> &'x [u8] {
        match &self.body {
            PartType::Text(text) | PartType::Html(text) => text.as_bytes(),
            PartType::Binary(bin) | PartType::InlineBinary(bin) => bin.as_ref(),
            PartType::Message(message) => message.raw_message.as_ref(),
            PartType::Multipart(_) => b"",
        }
    }

    /// Returns the body part's contents as a `str`
    pub fn text_contents(&'x self) -> Option<&'x str> {
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
    pub fn message(&'x self) -> Option<&Message<'x>> {
        if let PartType::Message(message) = &self.body {
            Some(message)
        } else {
            None
        }
    }

    /// Returns the sub parts ids of a MIME part
    pub fn sub_parts(&'x self) -> Option<&[MessagePartId]> {
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
    pub fn into_owned(self) -> MessagePart<'static> {
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
        fmt.write_str(self.text_contents().unwrap_or("[no contents]"))
    }
}

impl<'x> MimeHeaders<'x> for MessagePart<'x> {
    fn content_description(&self) -> Option<&str> {
        self.headers
            .rfc(&RfcHeader::ContentDescription)
            .and_then(|header| header.as_text())
    }

    fn content_disposition(&self) -> Option<&ContentType> {
        self.headers
            .rfc(&RfcHeader::ContentDisposition)
            .and_then(|header| header.as_content_type_ref())
    }

    fn content_id(&self) -> Option<&str> {
        self.headers
            .rfc(&RfcHeader::ContentId)
            .and_then(|header| header.as_text())
    }

    fn content_transfer_encoding(&self) -> Option<&str> {
        self.headers
            .rfc(&RfcHeader::ContentTransferEncoding)
            .and_then(|header| header.as_text())
    }

    fn content_type(&self) -> Option<&ContentType> {
        self.headers
            .rfc(&RfcHeader::ContentType)
            .and_then(|header| header.as_content_type_ref())
    }

    fn content_language(&self) -> &HeaderValue {
        self.headers
            .rfc(&RfcHeader::ContentLanguage)
            .unwrap_or(&HeaderValue::Empty)
    }

    fn content_location(&self) -> Option<&str> {
        self.headers
            .rfc(&RfcHeader::ContentLocation)
            .and_then(|header| header.as_text())
    }
}

/// An RFC2047 Content-Type or RFC2183 Content-Disposition MIME header field.
impl<'x> ContentType<'x> {
    /// Returns the type
    pub fn ctype(&self) -> &str {
        &self.c_type
    }

    /// Returns the sub-type
    pub fn subtype(&self) -> Option<&str> {
        self.c_subtype.as_ref()?.as_ref().into()
    }

    /// Returns an attribute by name
    pub fn attribute(&self, name: &str) -> Option<&str> {
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
    pub fn attributes(&self) -> Option<&[(Cow<str>, Cow<str>)]> {
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

/// A Received header
impl<'x> Received<'x> {
    pub fn into_owned(self) -> Received<'static> {
        Received {
            from: self.from.map(|s| s.into_owned()),
            from_ip: self.from_ip,
            from_iprev: self.from_iprev.map(|s| s.into_owned().into()),
            by: self.by.map(|s| s.into_owned()),
            for_: self.for_.map(|s| s.into_owned().into()),
            with: self.with,
            tls_version: self.tls_version,
            tls_cipher: self.tls_cipher.map(|s| s.into_owned().into()),
            id: self.id.map(|s| s.into_owned().into()),
            ident: self.ident.map(|s| s.into_owned().into()),
            helo: self.helo.map(|s| s.into_owned()),
            helo_cmd: self.helo_cmd,
            via: self.via.map(|s| s.into_owned().into()),
            date: self.date,
        }
    }
}

/// A hostname or IP address.
impl<'x> Host<'x> {
    pub fn into_owned(self) -> Host<'static> {
        match self {
            Host::Name(name) => Host::Name(name.into_owned().into()),
            Host::IpAddr(ip) => Host::IpAddr(ip),
        }
    }
}

impl<'x> GetHeader for Vec<Header<'x>> {
    fn rfc(&self, name: &RfcHeader) -> Option<&HeaderValue<'x>> {
        self.iter()
            .rev()
            .find(|header| matches!(&header.name, HeaderName::Rfc(rfc_name) if rfc_name == name))
            .map(|header| &header.value)
    }

    fn header(&self, name: &str) -> Option<&Header> {
        self.iter()
            .rev()
            .find(|header| header.name.as_str().eq_ignore_ascii_case(name))
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

impl From<DateTime> for i64 {
    fn from(value: DateTime) -> Self {
        value.to_timestamp()
    }
}

impl Display for RfcHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
