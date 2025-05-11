/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::fmt::Display;

use rkyv::{string::ArchivedString, vec::ArchivedVec};

use crate::{
    Addr, Address, ArchivedAddr, ArchivedAddress, ArchivedContentType, ArchivedDateTime,
    ArchivedEncoding, ArchivedGreeting, ArchivedGroup, ArchivedHeader, ArchivedHeaderName,
    ArchivedHeaderValue, ArchivedHost, ArchivedProtocol, ArchivedReceived, ArchivedTlsVersion,
    Attribute, ContentType, DateTime, Greeting, Group, HeaderName, HeaderValue, Host, Protocol,
    Received, TlsVersion,
};

pub trait ArchivedGetHeader<'x> {
    fn header_value(&self, name: &ArchivedHeaderName<'x>) -> Option<&ArchivedHeaderValue<'x>>;
    fn header(&self, name: impl Into<ArchivedHeaderName<'x>>) -> Option<&ArchivedHeader<'x>>;
}

impl<'x> ArchivedGetHeader<'x> for ArchivedVec<ArchivedHeader<'x>> {
    fn header_value(&self, name: &ArchivedHeaderName<'x>) -> Option<&ArchivedHeaderValue<'x>> {
        self.iter()
            .rev()
            .find(move |header| &header.name == name)
            .map(|header| &header.value)
    }

    fn header(&self, name: impl Into<ArchivedHeaderName<'x>>) -> Option<&ArchivedHeader<'x>> {
        let name = name.into();
        self.iter().rev().find(|header| header.name == name)
    }
}

impl ArchivedHeaderName<'_> {
    pub fn id(&self) -> u8 {
        match self {
            ArchivedHeaderName::Subject => 0,
            ArchivedHeaderName::From => 1,
            ArchivedHeaderName::To => 2,
            ArchivedHeaderName::Cc => 3,
            ArchivedHeaderName::Date => 4,
            ArchivedHeaderName::Bcc => 5,
            ArchivedHeaderName::ReplyTo => 6,
            ArchivedHeaderName::Sender => 7,
            ArchivedHeaderName::Comments => 8,
            ArchivedHeaderName::InReplyTo => 9,
            ArchivedHeaderName::Keywords => 10,
            ArchivedHeaderName::Received => 11,
            ArchivedHeaderName::MessageId => 12,
            ArchivedHeaderName::References => 13,
            ArchivedHeaderName::ReturnPath => 14,
            ArchivedHeaderName::MimeVersion => 15,
            ArchivedHeaderName::ContentDescription => 16,
            ArchivedHeaderName::ContentId => 17,
            ArchivedHeaderName::ContentLanguage => 18,
            ArchivedHeaderName::ContentLocation => 19,
            ArchivedHeaderName::ContentTransferEncoding => 20,
            ArchivedHeaderName::ContentType => 21,
            ArchivedHeaderName::ContentDisposition => 22,
            ArchivedHeaderName::ResentTo => 23,
            ArchivedHeaderName::ResentFrom => 24,
            ArchivedHeaderName::ResentBcc => 25,
            ArchivedHeaderName::ResentCc => 26,
            ArchivedHeaderName::ResentSender => 27,
            ArchivedHeaderName::ResentDate => 28,
            ArchivedHeaderName::ResentMessageId => 29,
            ArchivedHeaderName::ListArchive => 30,
            ArchivedHeaderName::ListHelp => 31,
            ArchivedHeaderName::ListId => 32,
            ArchivedHeaderName::ListOwner => 33,
            ArchivedHeaderName::ListPost => 34,
            ArchivedHeaderName::ListSubscribe => 35,
            ArchivedHeaderName::ListUnsubscribe => 36,
            ArchivedHeaderName::Other(_) => 37,
            ArchivedHeaderName::ArcAuthenticationResults => 38,
            ArchivedHeaderName::ArcMessageSignature => 39,
            ArchivedHeaderName::ArcSeal => 40,
            ArchivedHeaderName::DkimSignature => 41,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ArchivedHeaderName::Subject => "Subject",
            ArchivedHeaderName::From => "From",
            ArchivedHeaderName::To => "To",
            ArchivedHeaderName::Cc => "Cc",
            ArchivedHeaderName::Date => "Date",
            ArchivedHeaderName::Bcc => "Bcc",
            ArchivedHeaderName::ReplyTo => "Reply-To",
            ArchivedHeaderName::Sender => "Sender",
            ArchivedHeaderName::Comments => "Comments",
            ArchivedHeaderName::InReplyTo => "In-Reply-To",
            ArchivedHeaderName::Keywords => "Keywords",
            ArchivedHeaderName::Received => "Received",
            ArchivedHeaderName::MessageId => "Message-ID",
            ArchivedHeaderName::References => "References",
            ArchivedHeaderName::ReturnPath => "Return-Path",
            ArchivedHeaderName::MimeVersion => "MIME-Version",
            ArchivedHeaderName::ContentDescription => "Content-Description",
            ArchivedHeaderName::ContentId => "Content-ID",
            ArchivedHeaderName::ContentLanguage => "Content-Language",
            ArchivedHeaderName::ContentLocation => "Content-Location",
            ArchivedHeaderName::ContentTransferEncoding => "Content-Transfer-Encoding",
            ArchivedHeaderName::ContentType => "Content-Type",
            ArchivedHeaderName::ContentDisposition => "Content-Disposition",
            ArchivedHeaderName::ResentTo => "Resent-To",
            ArchivedHeaderName::ResentFrom => "Resent-From",
            ArchivedHeaderName::ResentBcc => "Resent-Bcc",
            ArchivedHeaderName::ResentCc => "Resent-Cc",
            ArchivedHeaderName::ResentSender => "Resent-Sender",
            ArchivedHeaderName::ResentDate => "Resent-Date",
            ArchivedHeaderName::ResentMessageId => "Resent-Message-ID",
            ArchivedHeaderName::ListArchive => "List-Archive",
            ArchivedHeaderName::ListHelp => "List-Help",
            ArchivedHeaderName::ListId => "List-ID",
            ArchivedHeaderName::ListOwner => "List-Owner",
            ArchivedHeaderName::ListPost => "List-Post",
            ArchivedHeaderName::ListSubscribe => "List-Subscribe",
            ArchivedHeaderName::ListUnsubscribe => "List-Unsubscribe",
            ArchivedHeaderName::ArcAuthenticationResults => "ARC-Authentication-Results",
            ArchivedHeaderName::ArcMessageSignature => "ARC-Message-Signature",
            ArchivedHeaderName::ArcSeal => "ARC-Seal",
            ArchivedHeaderName::DkimSignature => "DKIM-Signature",
            ArchivedHeaderName::Other(v) => v.as_str(),
        }
    }

    pub fn is_mime_header(&self) -> bool {
        matches!(
            self,
            ArchivedHeaderName::ContentDescription
                | ArchivedHeaderName::ContentId
                | ArchivedHeaderName::ContentLanguage
                | ArchivedHeaderName::ContentLocation
                | ArchivedHeaderName::ContentTransferEncoding
                | ArchivedHeaderName::ContentType
                | ArchivedHeaderName::ContentDisposition
        )
    }
}

impl ArchivedEncoding {
    pub fn id(&self) -> u8 {
        match self {
            ArchivedEncoding::None => 0,
            ArchivedEncoding::QuotedPrintable => 1,
            ArchivedEncoding::Base64 => 2,
        }
    }
}

impl Display for ArchivedHeaderName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq for ArchivedHeaderName<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Other(a), Self::Other(b)) => a.eq_ignore_ascii_case(b),
            (Self::Subject, Self::Subject) => true,
            (Self::From, Self::From) => true,
            (Self::To, Self::To) => true,
            (Self::Cc, Self::Cc) => true,
            (Self::Date, Self::Date) => true,
            (Self::Bcc, Self::Bcc) => true,
            (Self::ReplyTo, Self::ReplyTo) => true,
            (Self::Sender, Self::Sender) => true,
            (Self::Comments, Self::Comments) => true,
            (Self::InReplyTo, Self::InReplyTo) => true,
            (Self::Keywords, Self::Keywords) => true,
            (Self::Received, Self::Received) => true,
            (Self::MessageId, Self::MessageId) => true,
            (Self::References, Self::References) => true,
            (Self::ReturnPath, Self::ReturnPath) => true,
            (Self::MimeVersion, Self::MimeVersion) => true,
            (Self::ContentDescription, Self::ContentDescription) => true,
            (Self::ContentId, Self::ContentId) => true,
            (Self::ContentLanguage, Self::ContentLanguage) => true,
            (Self::ContentLocation, Self::ContentLocation) => true,
            (Self::ContentTransferEncoding, Self::ContentTransferEncoding) => true,
            (Self::ContentType, Self::ContentType) => true,
            (Self::ContentDisposition, Self::ContentDisposition) => true,
            (Self::ResentTo, Self::ResentTo) => true,
            (Self::ResentFrom, Self::ResentFrom) => true,
            (Self::ResentBcc, Self::ResentBcc) => true,
            (Self::ResentCc, Self::ResentCc) => true,
            (Self::ResentSender, Self::ResentSender) => true,
            (Self::ResentDate, Self::ResentDate) => true,
            (Self::ResentMessageId, Self::ResentMessageId) => true,
            (Self::ListArchive, Self::ListArchive) => true,
            (Self::ListHelp, Self::ListHelp) => true,
            (Self::ListId, Self::ListId) => true,
            (Self::ListOwner, Self::ListOwner) => true,
            (Self::ListPost, Self::ListPost) => true,
            (Self::ListSubscribe, Self::ListSubscribe) => true,
            (Self::ListUnsubscribe, Self::ListUnsubscribe) => true,
            (Self::ArcAuthenticationResults, Self::ArcAuthenticationResults) => true,
            (Self::ArcMessageSignature, Self::ArcMessageSignature) => true,
            (Self::ArcSeal, Self::ArcSeal) => true,
            (Self::DkimSignature, Self::DkimSignature) => true,
            _ => false,
        }
    }
}

impl std::hash::Hash for ArchivedHeaderName<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ArchivedHeaderName::Other(value) => {
                for ch in value.as_bytes() {
                    ch.to_ascii_lowercase().hash(state)
                }
            }
            _ => self.id().hash(state),
        }
    }
}

impl Eq for ArchivedHeaderName<'_> {}

impl From<&ArchivedHeaderName<'_>> for HeaderName<'static> {
    fn from(value: &ArchivedHeaderName<'_>) -> Self {
        match value {
            ArchivedHeaderName::Subject => HeaderName::Subject,
            ArchivedHeaderName::From => HeaderName::From,
            ArchivedHeaderName::To => HeaderName::To,
            ArchivedHeaderName::Cc => HeaderName::Cc,
            ArchivedHeaderName::Date => HeaderName::Date,
            ArchivedHeaderName::Bcc => HeaderName::Bcc,
            ArchivedHeaderName::ReplyTo => HeaderName::ReplyTo,
            ArchivedHeaderName::Sender => HeaderName::Sender,
            ArchivedHeaderName::Comments => HeaderName::Comments,
            ArchivedHeaderName::InReplyTo => HeaderName::InReplyTo,
            ArchivedHeaderName::Keywords => HeaderName::Keywords,
            ArchivedHeaderName::Received => HeaderName::Received,
            ArchivedHeaderName::MessageId => HeaderName::MessageId,
            ArchivedHeaderName::References => HeaderName::References,
            ArchivedHeaderName::ReturnPath => HeaderName::ReturnPath,
            ArchivedHeaderName::MimeVersion => HeaderName::MimeVersion,
            ArchivedHeaderName::ContentDescription => HeaderName::ContentDescription,
            ArchivedHeaderName::ContentId => HeaderName::ContentId,
            ArchivedHeaderName::ContentLanguage => HeaderName::ContentLanguage,
            ArchivedHeaderName::ContentLocation => HeaderName::ContentLocation,
            ArchivedHeaderName::ContentTransferEncoding => HeaderName::ContentTransferEncoding,
            ArchivedHeaderName::ContentType => HeaderName::ContentType,
            ArchivedHeaderName::ContentDisposition => HeaderName::ContentDisposition,
            ArchivedHeaderName::ResentTo => HeaderName::ResentTo,
            ArchivedHeaderName::ResentFrom => HeaderName::ResentFrom,
            ArchivedHeaderName::ResentBcc => HeaderName::ResentBcc,
            ArchivedHeaderName::ResentCc => HeaderName::ResentCc,
            ArchivedHeaderName::ResentSender => HeaderName::ResentSender,
            ArchivedHeaderName::ResentDate => HeaderName::ResentDate,
            ArchivedHeaderName::ResentMessageId => HeaderName::ResentMessageId,
            ArchivedHeaderName::ListArchive => HeaderName::ListArchive,
            ArchivedHeaderName::ListHelp => HeaderName::ListHelp,
            ArchivedHeaderName::ListId => HeaderName::ListId,
            ArchivedHeaderName::ListOwner => HeaderName::ListOwner,
            ArchivedHeaderName::ListPost => HeaderName::ListPost,
            ArchivedHeaderName::ListSubscribe => HeaderName::ListSubscribe,
            ArchivedHeaderName::ListUnsubscribe => HeaderName::ListUnsubscribe,
            ArchivedHeaderName::Other(other) => HeaderName::Other(other.to_string().into()),
            ArchivedHeaderName::ArcAuthenticationResults => HeaderName::ArcAuthenticationResults,
            ArchivedHeaderName::ArcMessageSignature => HeaderName::ArcMessageSignature,
            ArchivedHeaderName::ArcSeal => HeaderName::ArcSeal,
            ArchivedHeaderName::DkimSignature => HeaderName::DkimSignature,
        }
    }
}

impl From<&ArchivedHeaderValue<'_>> for HeaderValue<'static> {
    fn from(value: &ArchivedHeaderValue<'_>) -> Self {
        match value {
            ArchivedHeaderValue::Text(s) => HeaderValue::Text(s.to_string().into()),
            ArchivedHeaderValue::TextList(list) => {
                HeaderValue::TextList(list.iter().map(|s| s.to_string().into()).collect())
            }
            ArchivedHeaderValue::DateTime(d) => HeaderValue::DateTime(d.into()),
            ArchivedHeaderValue::ContentType(ct) => HeaderValue::ContentType(ct.into()),
            ArchivedHeaderValue::Empty => HeaderValue::Empty,
            ArchivedHeaderValue::Address(a) => HeaderValue::Address(a.into()),
            ArchivedHeaderValue::Received(r) => HeaderValue::Received(Box::new(r.as_ref().into())),
        }
    }
}

impl From<&ArchivedAddress<'_>> for Address<'static> {
    fn from(value: &ArchivedAddress<'_>) -> Self {
        match value {
            ArchivedAddress::List(list) => Address::List(list.iter().map(Into::into).collect()),
            ArchivedAddress::Group(groups) => {
                Address::Group(groups.iter().map(Into::into).collect())
            }
        }
    }
}

impl From<&ArchivedContentType<'_>> for ContentType<'static> {
    fn from(value: &ArchivedContentType<'_>) -> Self {
        ContentType {
            c_type: value.c_type.to_string().into(),
            c_subtype: value.subtype().map(|s| s.to_string().into()),
            attributes: value.attributes.as_ref().map(|attrs| {
                attrs
                    .iter()
                    .map(|a| Attribute {
                        name: a.name.to_string().into(),
                        value: a.value.to_string().into(),
                    })
                    .collect()
            }),
        }
    }
}

impl From<&ArchivedGroup<'_>> for Group<'static> {
    fn from(value: &ArchivedGroup<'_>) -> Self {
        Group {
            name: value.name.as_ref().map(|s| s.to_string().into()),
            addresses: value.addresses.iter().map(|a| a.into()).collect(),
        }
    }
}

impl From<&ArchivedAddr<'_>> for Addr<'static> {
    fn from(value: &ArchivedAddr<'_>) -> Self {
        Addr {
            name: value.name().map(|s| s.to_string().into()),
            address: value.address().map(|s| s.to_string().into()),
        }
    }
}

impl From<&ArchivedDateTime> for DateTime {
    fn from(value: &ArchivedDateTime) -> Self {
        DateTime {
            year: value.year.to_native(),
            month: value.month,
            day: value.day,
            hour: value.hour,
            minute: value.minute,
            second: value.second,
            tz_before_gmt: value.tz_before_gmt,
            tz_hour: value.tz_hour,
            tz_minute: value.tz_minute,
        }
    }
}

impl From<&ArchivedReceived<'_>> for Received<'static> {
    fn from(value: &ArchivedReceived<'_>) -> Self {
        Received {
            from: value.from.as_ref().map(|s| s.into()),
            from_ip: value.from_ip.as_ref().map(|s| s.as_ipaddr()),
            from_iprev: value.from_iprev.as_ref().map(|s| s.to_string().into()),
            by: value.by.as_ref().map(|s| s.into()),
            for_: value.for_.as_ref().map(|s| s.to_string().into()),
            with: value.with.as_ref().map(|s| s.into()),
            tls_version: value.tls_version.as_ref().map(|s| s.into()),
            tls_cipher: value.tls_cipher.as_ref().map(|s| s.to_string().into()),
            id: value.id.as_ref().map(|s| s.to_string().into()),
            ident: value.ident.as_ref().map(|s| s.to_string().into()),
            helo: value.helo.as_ref().map(|s| s.into()),
            helo_cmd: value.helo_cmd.as_ref().map(|s| s.into()),
            via: value.via.as_ref().map(|s| s.to_string().into()),
            date: value.date.as_ref().map(|s| s.into()),
        }
    }
}

impl From<&ArchivedProtocol> for Protocol {
    fn from(value: &ArchivedProtocol) -> Self {
        match value {
            ArchivedProtocol::SMTP => Protocol::SMTP,
            ArchivedProtocol::ESMTP => Protocol::ESMTP,
            ArchivedProtocol::ESMTPA => Protocol::ESMTPA,
            ArchivedProtocol::ESMTPS => Protocol::ESMTPS,
            ArchivedProtocol::ESMTPSA => Protocol::ESMTPSA,
            ArchivedProtocol::LMTP => Protocol::LMTP,
            ArchivedProtocol::LMTPA => Protocol::LMTPA,
            ArchivedProtocol::LMTPS => Protocol::LMTPS,
            ArchivedProtocol::LMTPSA => Protocol::LMTPSA,
            ArchivedProtocol::MMS => Protocol::MMS,
            ArchivedProtocol::UTF8SMTP => Protocol::UTF8SMTP,
            ArchivedProtocol::UTF8SMTPA => Protocol::UTF8SMTPA,
            ArchivedProtocol::UTF8SMTPS => Protocol::UTF8SMTPS,
            ArchivedProtocol::UTF8SMTPSA => Protocol::UTF8SMTPSA,
            ArchivedProtocol::UTF8LMTP => Protocol::UTF8LMTP,
            ArchivedProtocol::UTF8LMTPA => Protocol::UTF8LMTPA,
            ArchivedProtocol::UTF8LMTPS => Protocol::UTF8LMTPS,
            ArchivedProtocol::UTF8LMTPSA => Protocol::UTF8LMTPSA,
            ArchivedProtocol::HTTP => Protocol::HTTP,
            ArchivedProtocol::HTTPS => Protocol::HTTPS,
            ArchivedProtocol::IMAP => Protocol::IMAP,
            ArchivedProtocol::POP3 => Protocol::POP3,
            ArchivedProtocol::Local => Protocol::Local,
        }
    }
}

impl From<&ArchivedGreeting> for Greeting {
    fn from(value: &ArchivedGreeting) -> Self {
        match value {
            ArchivedGreeting::Helo => Greeting::Helo,
            ArchivedGreeting::Ehlo => Greeting::Ehlo,
            ArchivedGreeting::Lhlo => Greeting::Lhlo,
        }
    }
}

impl From<&ArchivedTlsVersion> for TlsVersion {
    fn from(value: &ArchivedTlsVersion) -> Self {
        match value {
            ArchivedTlsVersion::SSLv2 => TlsVersion::SSLv2,
            ArchivedTlsVersion::SSLv3 => TlsVersion::SSLv3,
            ArchivedTlsVersion::TLSv1_0 => TlsVersion::TLSv1_0,
            ArchivedTlsVersion::TLSv1_1 => TlsVersion::TLSv1_1,
            ArchivedTlsVersion::TLSv1_2 => TlsVersion::TLSv1_2,
            ArchivedTlsVersion::TLSv1_3 => TlsVersion::TLSv1_3,
            ArchivedTlsVersion::DTLSv1_0 => TlsVersion::DTLSv1_0,
            ArchivedTlsVersion::DTLSv1_2 => TlsVersion::DTLSv1_2,
            ArchivedTlsVersion::DTLSv1_3 => TlsVersion::DTLSv1_3,
        }
    }
}

impl From<&ArchivedHost<'_>> for Host<'static> {
    fn from(value: &ArchivedHost<'_>) -> Self {
        match value {
            ArchivedHost::Name(name) => Host::Name(name.to_string().into()),
            ArchivedHost::IpAddr(ip) => Host::IpAddr(ip.as_ipaddr()),
        }
    }
}

impl<'x> ArchivedAddress<'x> {
    pub fn iter(
        &self,
    ) -> Box<dyn DoubleEndedIterator<Item = &ArchivedAddr<'x>> + '_ + Sync + Send> {
        match self {
            ArchivedAddress::List(list) => Box::new(list.iter()),
            ArchivedAddress::Group(group) => {
                Box::new(group.iter().flat_map(|group| group.addresses.iter()))
            }
        }
    }
}

impl ArchivedAddr<'_> {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }
}

impl ArchivedContentType<'_> {
    pub fn ctype(&self) -> &str {
        &self.c_type
    }

    pub fn subtype(&self) -> Option<&str> {
        self.c_subtype.as_deref()
    }

    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .as_ref()?
            .iter()
            .find(|k| k.name == name)?
            .value
            .as_ref()
            .into()
    }
}

impl<'x> ArchivedHeaderValue<'x> {
    pub fn as_text(&self) -> Option<&str> {
        match *self {
            ArchivedHeaderValue::Text(ref s) => Some(s),
            ArchivedHeaderValue::TextList(ref l) => l.last().map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn as_content_type(&self) -> Option<&ArchivedContentType<'x>> {
        match self {
            ArchivedHeaderValue::ContentType(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_text_list(&self) -> Option<&[ArchivedString]> {
        match *self {
            ArchivedHeaderValue::Text(ref s) => Some(std::slice::from_ref(s)),
            ArchivedHeaderValue::TextList(ref l) => Some(l.as_slice()),
            _ => None,
        }
    }

    pub fn as_datetime(&self) -> Option<&ArchivedDateTime> {
        match self {
            ArchivedHeaderValue::DateTime(d) => Some(d),
            _ => None,
        }
    }
}
