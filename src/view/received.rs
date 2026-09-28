/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::Resolver;
use crate::{
    DateTime,
    store::{HostEntry, ReceivedEntry},
};
use std::{fmt, net::IpAddr};

/// A parsed Received trace field (RFC 5321 section 4.4). Every clause is
/// optional; values compare by content.
#[derive(Clone, Copy)]
pub struct Received<'m> {
    resolver: Resolver<'m>,
    entry: &'m ReceivedEntry,
}

/// A host name or an IP address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host<'m> {
    /// A host name, or an address literal that is not an IP address.
    Name(&'m str),
    /// An IPv4 or IPv6 address.
    IpAddr(IpAddr),
}

/// The TLS version of a Received `with` clause comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TlsVersion {
    /// SSL 2.
    SSLv2,
    /// SSL 3.
    SSLv3,
    /// TLS 1.0.
    TLSv1_0,
    /// TLS 1.1.
    TLSv1_1,
    /// TLS 1.2.
    TLSv1_2,
    /// TLS 1.3.
    TLSv1_3,
    /// DTLS 1.0.
    DTLSv1_0,
    /// DTLS 1.2.
    DTLSv1_2,
    /// DTLS 1.3.
    DTLSv1_3,
}

/// The greeting command of a Received `from` clause comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Greeting {
    /// `HELO` (SMTP).
    Helo,
    /// `EHLO` (ESMTP).
    Ehlo,
    /// `LHLO` (LMTP).
    Lhlo,
}

/// The protocol of a Received `with` clause (the IANA mail transmission
/// types of RFC 3848 and later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    /// `SMTP`.
    SMTP,
    /// `ESMTP`.
    ESMTP,
    /// `ESMTPA`: ESMTP with authentication.
    ESMTPA,
    /// `ESMTPS`: ESMTP with TLS.
    ESMTPS,
    /// `ESMTPSA`: ESMTP with TLS and authentication.
    ESMTPSA,
    /// `LMTP`.
    LMTP,
    /// `LMTPA`: LMTP with authentication.
    LMTPA,
    /// `LMTPS`: LMTP with TLS.
    LMTPS,
    /// `LMTPSA`: LMTP with TLS and authentication.
    LMTPSA,
    /// `MMS`.
    MMS,
    /// `UTF8SMTP`.
    UTF8SMTP,
    /// `UTF8SMTPA`.
    UTF8SMTPA,
    /// `UTF8SMTPS`.
    UTF8SMTPS,
    /// `UTF8SMTPSA`.
    UTF8SMTPSA,
    /// `UTF8LMTP`.
    UTF8LMTP,
    /// `UTF8LMTPA`.
    UTF8LMTPA,
    /// `UTF8LMTPS`.
    UTF8LMTPS,
    /// `UTF8LMTPSA`.
    UTF8LMTPSA,
    /// `HTTP`.
    HTTP,
    /// `HTTPS`.
    HTTPS,
    /// `IMAP`.
    IMAP,
    /// `POP3`.
    POP3,
    /// `Local`.
    Local,
}

impl<'m> Received<'m> {
    pub(crate) fn new(resolver: Resolver<'m>, entry: &'m ReceivedEntry) -> Self {
        Received { resolver, entry }
    }

    fn host(&self, host: HostEntry) -> Option<Host<'m>> {
        match host {
            HostEntry::None => None,
            HostEntry::Name(name) => Some(Host::Name(self.resolver.str(name))),
            HostEntry::Ip(ip) => Some(Host::IpAddr(ip)),
        }
    }

    /// Host that sent the message.
    pub fn from(&self) -> Option<Host<'m>> {
        self.host(self.entry.from)
    }

    /// IP address of the sending host: an address literal or an address in
    /// a comment.
    pub fn from_ip(&self) -> Option<IpAddr> {
        self.entry.from_ip
    }

    /// The name the sending address resolved to, from a comment.
    pub fn from_iprev(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.from_iprev)
    }

    /// Host that received the message.
    pub fn by(&self) -> Option<Host<'m>> {
        self.host(self.entry.by)
    }

    /// The recipient of the `for` clause.
    pub fn for_(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.for_)
    }

    /// The protocol of the `with` clause.
    pub fn with(&self) -> Option<Protocol> {
        self.entry.with
    }

    /// The TLS version named in a comment.
    pub fn tls_version(&self) -> Option<TlsVersion> {
        self.entry.tls_version
    }

    /// The TLS cipher suite, when one is named.
    pub fn tls_cipher(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.tls_cipher)
    }

    /// The identifier of the `id` clause.
    pub fn id(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.id)
    }

    /// The ident (RFC 1413) user, from a comment.
    pub fn ident(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.ident)
    }

    /// The name the sender greeted with (`helo=` in a comment).
    pub fn helo(&self) -> Option<Host<'m>> {
        self.host(self.entry.helo)
    }

    /// The greeting command named in a comment.
    pub fn helo_cmd(&self) -> Option<Greeting> {
        self.entry.helo_cmd
    }

    /// The link of the `via` clause.
    pub fn via(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.via)
    }

    /// The date after the `;`.
    pub fn date(&self) -> Option<DateTime> {
        self.entry.date
    }
}

impl fmt::Debug for Received<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Received")
            .field("from", &self.from())
            .field("from_ip", &self.from_ip())
            .field("from_iprev", &self.from_iprev())
            .field("by", &self.by())
            .field("for", &self.for_())
            .field("with", &self.with())
            .field("tls_version", &self.tls_version())
            .field("tls_cipher", &self.tls_cipher())
            .field("id", &self.id())
            .field("ident", &self.ident())
            .field("helo", &self.helo())
            .field("helo_cmd", &self.helo_cmd())
            .field("via", &self.via())
            .field("date", &self.date())
            .finish()
    }
}

impl PartialEq for Received<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.from() == other.from()
            && self.from_ip() == other.from_ip()
            && self.from_iprev() == other.from_iprev()
            && self.by() == other.by()
            && self.for_() == other.for_()
            && self.with() == other.with()
            && self.tls_version() == other.tls_version()
            && self.tls_cipher() == other.tls_cipher()
            && self.id() == other.id()
            && self.ident() == other.ident()
            && self.helo() == other.helo()
            && self.helo_cmd() == other.helo_cmd()
            && self.via() == other.via()
            && self.date() == other.date()
    }
}

impl Eq for Received<'_> {}

impl TlsVersion {
    /// The version as usually written (`TLSv1.3`).
    pub fn as_str(&self) -> &'static str {
        match self {
            TlsVersion::SSLv2 => "SSLv2",
            TlsVersion::SSLv3 => "SSLv3",
            TlsVersion::TLSv1_0 => "TLSv1.0",
            TlsVersion::TLSv1_1 => "TLSv1.1",
            TlsVersion::TLSv1_2 => "TLSv1.2",
            TlsVersion::TLSv1_3 => "TLSv1.3",
            TlsVersion::DTLSv1_0 => "DTLSv1.0",
            TlsVersion::DTLSv1_2 => "DTLSv1.2",
            TlsVersion::DTLSv1_3 => "DTLSv1.3",
        }
    }
}

impl Greeting {
    /// The command in uppercase.
    pub fn as_str(&self) -> &'static str {
        match self {
            Greeting::Helo => "HELO",
            Greeting::Ehlo => "EHLO",
            Greeting::Lhlo => "LHLO",
        }
    }
}

impl Protocol {
    /// The protocol name as written in a `with` clause.
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::SMTP => "SMTP",
            Protocol::LMTP => "LMTP",
            Protocol::ESMTP => "ESMTP",
            Protocol::ESMTPS => "ESMTPS",
            Protocol::ESMTPA => "ESMTPA",
            Protocol::ESMTPSA => "ESMTPSA",
            Protocol::LMTPA => "LMTPA",
            Protocol::LMTPS => "LMTPS",
            Protocol::LMTPSA => "LMTPSA",
            Protocol::UTF8SMTP => "UTF8SMTP",
            Protocol::UTF8SMTPA => "UTF8SMTPA",
            Protocol::UTF8SMTPS => "UTF8SMTPS",
            Protocol::UTF8SMTPSA => "UTF8SMTPSA",
            Protocol::UTF8LMTP => "UTF8LMTP",
            Protocol::UTF8LMTPA => "UTF8LMTPA",
            Protocol::UTF8LMTPS => "UTF8LMTPS",
            Protocol::UTF8LMTPSA => "UTF8LMTPSA",
            Protocol::HTTP => "HTTP",
            Protocol::HTTPS => "HTTPS",
            Protocol::IMAP => "IMAP",
            Protocol::POP3 => "POP3",
            Protocol::MMS => "MMS",
            Protocol::Local => "Local",
        }
    }
}

impl fmt::Display for Host<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Host::Name(name) => f.write_str(name),
            Host::IpAddr(ip) => ip.fmt(f),
        }
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Greeting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for TlsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
