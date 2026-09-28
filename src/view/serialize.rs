/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{
    Address, AddressList, AddressRun, ContentType, DecodeProblems, Group, Header, HeaderValue,
    Headers, Host, Mailbox, Message, MessagePart, MessageRef, PartFlags, PartKind, PartRole,
    Received, Source, TextList,
};
use crate::Encoding;
use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeStruct},
};
use std::cell::Cell;

struct Seq<I>(Cell<Option<I>>);

fn seq<I>(iter: I) -> Seq<I> {
    Seq(Cell::new(Some(iter)))
}

impl<I> Serialize for Seq<I>
where
    I: Iterator,
    I::Item: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.take().into_iter().flatten())
    }
}

struct Tagged<'a, T>(&'static str, &'a T);

impl<T: Serialize> Serialize for Tagged<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.0, self.1)?;
        map.end()
    }
}

impl Serialize for Message<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Message", 2)?;
        state.serialize_field("messages", &seq(self.messages()))?;
        state.serialize_field("parts", &seq(self.parts()))?;
        state.end()
    }
}

impl Serialize for MessageRef<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let ids = |parts: &mut dyn Iterator<Item = MessagePart<'_>>| {
            parts.map(|part| part.id()).collect::<Vec<_>>()
        };
        let mut state = serializer.serialize_struct("MessageRef", 8)?;
        state.serialize_field("id", &self.id())?;
        state.serialize_field("root", &self.root_part().id())?;
        state.serialize_field("container", &self.container().map(|part| part.id()))?;
        state.serialize_field("source", &self.source())?;
        state.serialize_field("text_body", &ids(&mut self.text_body()))?;
        state.serialize_field("html_body", &ids(&mut self.html_body()))?;
        state.serialize_field("attachments", &ids(&mut self.attachments()))?;
        state.serialize_field("has_attachments", &self.has_attachments())?;
        state.end()
    }
}

impl Serialize for Source {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Source::Raw => serializer.serialize_str("raw"),
            Source::Decoded(part) => Tagged("decoded", part).serialize(serializer),
        }
    }
}

impl Serialize for Encoding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Encoding::None => "none",
            Encoding::QuotedPrintable => "quoted-printable",
            Encoding::Base64 => "base64",
        })
    }
}

impl Serialize for PartFlags {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        seq(self.names()).serialize(serializer)
    }
}

impl Serialize for DecodeProblems {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        seq(self.names()).serialize(serializer)
    }
}

impl Serialize for PartKind<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            PartKind::Text => "text",
            PartKind::Html => "html",
            PartKind::Binary => "binary",
            PartKind::InlineBinary => "inline_binary",
            PartKind::Multipart => "multipart",
            PartKind::Message(_) => "message",
        })
    }
}

impl Serialize for PartRole {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            PartRole::Container => "container",
            PartRole::TextBody => "text_body",
            PartRole::HtmlBody => "html_body",
            PartRole::CopiedBody => "copied_body",
            PartRole::MediaBody => "media_body",
            PartRole::Attachment => "attachment",
            PartRole::Dropped => "dropped",
        })
    }
}

impl Serialize for MessagePart<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("MessagePart", 16)?;
        state.serialize_field("id", &self.id())?;
        state.serialize_field("message", &self.message().id())?;
        state.serialize_field("parent", &self.parent().map(|part| part.id()))?;
        state.serialize_field("kind", &self.kind())?;
        match self.kind() {
            PartKind::Multipart => {
                state.serialize_field("children", &seq(self.children().map(|part| part.id())))?
            }
            PartKind::Message(nested) => state.serialize_field("nested", &nested.id())?,
            _ => state.skip_field("children")?,
        }
        state.serialize_field("encoding", &self.encoding())?;
        state.serialize_field("flags", &self.flags())?;
        let roles = [
            (self.in_text_body(), "text_body"),
            (self.in_html_body(), "html_body"),
            (self.is_attachment(), "attachment"),
            (self.is_inline(), "inline"),
        ];
        state.serialize_field(
            "roles",
            &seq(roles.iter().filter(|(set, _)| *set).map(|(_, name)| *name)),
        )?;
        state.serialize_field("role", &self.role())?;
        state.serialize_field("offset_header", &self.offset_header())?;
        state.serialize_field("offset_body", &self.offset_body())?;
        state.serialize_field("offset_end", &self.offset_end())?;
        state.serialize_field("headers", &self.headers())?;
        match self.kind() {
            PartKind::Text | PartKind::Html => {
                let (text, problems) = self.text_checked().unwrap_or_default();
                state.serialize_field("text", &text)?;
                state.serialize_field("decode_problems", &problems)?;
            }
            PartKind::Binary | PartKind::InlineBinary => {
                let (decoded, problems) = self.decoded_checked();
                state.serialize_field("binary", &encodify::base64::STANDARD.encode(decoded))?;
                state.serialize_field("decode_problems", &problems)?;
            }
            PartKind::Multipart | PartKind::Message(_) => state.skip_field("text")?,
        }
        state.end()
    }
}

impl Serialize for Headers<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        seq(self.iter()).serialize(serializer)
    }
}

impl Serialize for Header<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Header", 5)?;
        state.serialize_field("name", self.raw_name())?;
        state.serialize_field("value", &self.value())?;
        state.serialize_field("offset_field", &self.offset_field())?;
        state.serialize_field("offset_start", &self.offset_start())?;
        state.serialize_field("offset_end", &self.offset_end())?;
        state.end()
    }
}

impl Serialize for HeaderValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            HeaderValue::Empty => serializer.serialize_none(),
            HeaderValue::Text(text) => Tagged("text", text).serialize(serializer),
            HeaderValue::TextList(list) => Tagged("text_list", list).serialize(serializer),
            HeaderValue::Address(list) => Tagged("address", list).serialize(serializer),
            HeaderValue::DateTime(date) => Tagged("date_time", date).serialize(serializer),
            HeaderValue::ContentType(content_type) => {
                Tagged("content_type", content_type).serialize(serializer)
            }
            HeaderValue::Received(received) => Tagged("received", received).serialize(serializer),
        }
    }
}

impl Serialize for TextList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        seq(self.iter()).serialize(serializer)
    }
}

impl Serialize for AddressList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl Serialize for Address<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Address::Mailbox(mailbox) => Tagged("mailbox", mailbox).serialize(serializer),
            Address::Group(group) => Tagged("group", group).serialize(serializer),
        }
    }
}

impl Serialize for Mailbox<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Mailbox", 2)?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("address", &self.address())?;
        state.end()
    }
}

impl Serialize for Group<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Group", 2)?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("mailboxes", &self.mailboxes())?;
        state.end()
    }
}

impl Serialize for AddressRun<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        seq(*self).serialize(serializer)
    }
}

impl Serialize for ContentType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("ContentType", 3)?;
        state.serialize_field("type", self.ctype())?;
        state.serialize_field("subtype", &self.subtype())?;
        state.serialize_field("attributes", &seq(self.attributes()))?;
        state.end()
    }
}

impl Serialize for Host<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Host::Name(name) => Tagged("name", name).serialize(serializer),
            Host::IpAddr(ip) => Tagged("ip", ip).serialize(serializer),
        }
    }
}

impl Serialize for Received<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Received", 14)?;
        state.serialize_field("from", &self.from())?;
        state.serialize_field("from_ip", &self.from_ip())?;
        state.serialize_field("from_iprev", &self.from_iprev())?;
        state.serialize_field("by", &self.by())?;
        state.serialize_field("for", &self.for_())?;
        state.serialize_field("with", &self.with())?;
        state.serialize_field("tls_version", &self.tls_version())?;
        state.serialize_field("tls_cipher", &self.tls_cipher())?;
        state.serialize_field("id", &self.id())?;
        state.serialize_field("ident", &self.ident())?;
        state.serialize_field("helo", &self.helo())?;
        state.serialize_field("helo_cmd", &self.helo_cmd())?;
        state.serialize_field("via", &self.via())?;
        state.serialize_field("date", &self.date())?;
        state.end()
    }
}
