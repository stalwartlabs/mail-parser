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

mod decoders;
mod parsers;

use std::{borrow::Cow, collections::HashMap};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Message<'x> {
    #[serde(borrow)]
    header: Box<MessageHeader<'x>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    html_body: Vec<BodyPart<'x>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    text_body: Vec<BodyPart<'x>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    attachments: Vec<AttachmentPart<'x>>,
}
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TextPart<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    header: Option<MimeHeader<'x>>,
    contents: Cow<'x, str>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BinaryPart<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    header: Option<MimeHeader<'x>>,
    #[serde(with = "serde_bytes")]
    #[serde(borrow)]
    contents: Cow<'x, [u8]>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BodyPart<'x> {
    Text(TextPart<'x>),
    InlineBinary(u32),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AttachmentPart<'x> {
    Text(TextPart<'x>),
    #[serde(borrow)]
    Binary(BinaryPart<'x>),
    #[serde(borrow)]
    InlineBinary(BinaryPart<'x>),
    Message(Message<'x>),
}

#[derive(PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct MessageHeader<'x> {
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub bcc: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub cc: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub comments: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub date: Option<DateTime>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub from: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub in_reply_to: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub keywords: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_archive: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_help: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_id: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_owner: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_post: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_subscribe: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub list_unsubscribe: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub message_id: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mime_version: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub received: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub references: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub reply_to: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub resent_bcc: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub resent_cc: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub resent_date: Option<Vec<DateTime>>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub resent_from: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub resent_message_id: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub resent_sender: Address<'x>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub resent_to: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub return_path: Option<Vec<Cow<'x, str>>>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub sender: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub subject: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Address::is_empty")]
    #[serde(default)]
    pub to: Address<'x>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_description: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_disposition: Option<ContentType<'x>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_id: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_transfer_encoding: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_type: Option<ContentType<'x>>,
    #[serde(borrow)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub others: HashMap<&'x str, Vec<Cow<'x, str>>>,
}

#[derive(PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct MimeHeader<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_description: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_disposition: Option<ContentType<'x>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_id: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_transfer_encoding: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_type: Option<ContentType<'x>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Addr<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    name: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    address: Option<Cow<'x, str>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Group<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    name: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    addresses: Vec<Addr<'x>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Address<'x> {
    Address(Addr<'x>),
    AddressList(Vec<Addr<'x>>),
    Group(Group<'x>),
    GroupList(Vec<Group<'x>>),
    Collection(Vec<Address<'x>>),
    Empty,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentType<'x> {
    c_type: Cow<'x, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    c_subtype: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    attributes: Option<HashMap<Cow<'x, str>, Cow<'x, str>>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

pub trait MimeFieldGet<'x> {
    fn get_content_description(&self) -> Option<&Cow<'x, str>>;
    fn get_content_disposition(&self) -> Option<&ContentType<'x>>;
    fn get_content_id(&self) -> Option<&Cow<'x, str>>;
    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>>;
    fn get_content_type(&self) -> Option<&ContentType<'x>>;
}

impl<'x> Message<'x> {
    pub fn get_bcc(&self) -> &Address<'x> {
        &self.header.bcc
    }

    pub fn get_cc(&self) -> &Address<'x> {
        &self.header.cc
    }

    pub fn get_comments(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.comments.as_ref()
    }

    pub fn get_date(&self) -> Option<&DateTime> {
        self.header.date.as_ref()
    }

    pub fn get_from(&self) -> &Address<'x> {
        &self.header.from
    }

    pub fn get_in_reply_to(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.in_reply_to.as_ref()
    }

    pub fn get_keywords(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.keywords.as_ref()
    }

    pub fn get_list_archive(&self) -> &Address<'x> {
        &self.header.list_archive
    }

    pub fn get_list_help(&self) -> &Address<'x> {
        &self.header.list_help
    }

    pub fn get_list_id(&self) -> &Address<'x> {
        &self.header.list_id
    }

    pub fn get_list_owner(&self) -> &Address<'x> {
        &self.header.list_owner
    }

    pub fn get_list_post(&self) -> &Address<'x> {
        &self.header.list_post
    }

    pub fn get_list_subscribe(&self) -> &Address<'x> {
        &self.header.list_subscribe
    }

    pub fn get_list_unsubscribe(&self) -> &Address<'x> {
        &self.header.list_unsubscribe
    }

    pub fn get_message_id(&self) -> Option<&Cow<'x, str>> {
        self.header.message_id.as_ref()
    }

    pub fn get_mime_version(&self) -> Option<&Cow<'x, str>> {
        self.header.mime_version.as_ref()
    }

    pub fn get_received(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.received.as_ref()
    }

    pub fn get_references(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.references.as_ref()
    }

    pub fn get_reply_to(&self) -> &Address<'x> {
        &self.header.reply_to
    }

    pub fn get_resent_bcc(&self) -> &Address<'x> {
        &self.header.bcc
    }

    pub fn get_resent_cc(&self) -> &Address<'x> {
        &self.header.resent_to
    }

    pub fn get_resent_date(&self) -> Option<&Vec<DateTime>> {
        self.header.resent_date.as_ref()
    }

    pub fn get_resent_from(&self) -> &Address<'x> {
        &self.header.resent_from
    }

    pub fn get_resent_message_id(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.resent_message_id.as_ref()
    }

    pub fn get_resent_sender(&self) -> &Address<'x> {
        &self.header.resent_sender
    }

    pub fn get_resent_to(&self) -> &Address<'x> {
        &self.header.resent_to
    }

    pub fn get_return_path(&self) -> Option<&Vec<Cow<'x, str>>> {
        self.header.return_path.as_ref()
    }

    pub fn get_sender(&self) -> &Address<'x> {
        &self.header.sender
    }

    pub fn get_subject(&self) -> Option<&Cow<'x, str>> {
        self.header.subject.as_ref()
    }

    pub fn get_to(&self) -> &Address<'x> {
        &self.header.to
    }

    pub fn get_header(&self, name: &str) -> Option<&Vec<Cow<'x, str>>> {
        self.header.others.get(name)
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
