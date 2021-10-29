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
