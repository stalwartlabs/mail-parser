use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap};

use crate::parsers::message_stream::MessageStream;

use super::fields::{
    address::Address, content_type::ContentType, date::DateTime, parse_unsupported, MessageField,
};

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

impl<'x> MessageHeader<'x> {
    pub fn new() -> MessageHeader<'x> {
        MessageHeader {
            ..Default::default()
        }
    }
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

impl<'x> MimeHeader<'x> {
    pub fn new() -> MimeHeader<'x> {
        MimeHeader {
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.content_description = None;
        self.content_disposition = None;
        self.content_id = None;
        self.content_transfer_encoding = None;
        self.content_type = None;
    }

    pub fn is_empty(&self) -> bool {
        self.content_description.is_none()
            && self.content_disposition.is_none()
            && self.content_id.is_none()
            && self.content_transfer_encoding.is_none()
            && self.content_type.is_none()
    }
}

enum HeaderParserResult<'x> {
    Supported(fn(&mut dyn MessageField<'x>, &MessageStream<'x>)),
    Unsupported(&'x [u8]),
    Lf,
    Eof,
}

pub fn parse_headers<'x>(header: &mut dyn MessageField<'x>, stream: &MessageStream<'x>) -> bool {
    loop {
        match parse_header_name(stream) {
            HeaderParserResult::Supported(fnc) => fnc(header, stream),
            HeaderParserResult::Unsupported(name) => parse_unsupported(header, stream, name),
            HeaderParserResult::Lf => return true,
            HeaderParserResult::Eof => return false,
            _ => return false,
        }
    }
}

fn parse_header_name<'x>(stream: &MessageStream<'x>) -> HeaderParserResult<'x> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut token_len: usize = 0;
    let mut token_hash: usize = 0;
    let mut last_ch: u8 = 0;

    while let Some(ch) = stream.next() {
        match ch {
            b':' => {
                if token_start != 0 {
                    let field = stream.get_bytes(token_start - 1, token_end).unwrap();

                    if (2..=25).contains(&token_len) {
                        token_hash += token_len
                            + unsafe {
                                *HDR_HASH.get_unchecked(last_ch.to_ascii_lowercase() as usize)
                            } as usize;

                        if (4..=61).contains(&token_hash) {
                            let token_hash = token_hash - 4;

                            if field.eq_ignore_ascii_case(unsafe {
                                HDR_NAMES.get_unchecked(token_hash)
                            }) {
                                return HeaderParserResult::Supported(unsafe {
                                    *HDR_FNCS.get_unchecked(token_hash)
                                });
                            }
                        }
                    }
                    return HeaderParserResult::Unsupported(field);
                }
            }
            b'\n' => {
                stream.rewind(1);
                return HeaderParserResult::Lf;
            }
            _ => {
                if !(*ch).is_ascii_whitespace() {
                    if token_start == 0 {
                        token_start = stream.get_pos();
                        token_end = token_start;
                    } else {
                        token_end = stream.get_pos();
                        last_ch = *ch;
                    }

                    if let 0 | 9 = token_len {
                        token_hash +=
                            unsafe { *HDR_HASH.get_unchecked((*ch).to_ascii_lowercase() as usize) }
                                as usize;
                    }
                    token_len += 1;
                }
            }
        }
    }
    HeaderParserResult::Eof
}

#[cfg(test)]
mod tests {
    use crate::parsers::{
        fields::{parse_from, parse_mime_version, parse_received, parse_subject},
        header::parse_header_name,
        message_stream::MessageStream,
    };

    use super::HeaderParserResult;

    #[test]
    fn header_name_parse() {
        let inputs = [
            ("From: ", HeaderParserResult::Supported(parse_from)),
            ("receiVED: ", HeaderParserResult::Supported(parse_received)),
            (
                " subject   : ",
                HeaderParserResult::Supported(parse_subject),
            ),
            (
                "X-Custom-Field : ",
                HeaderParserResult::Unsupported(b"X-Custom-Field"),
            ),
            (" T : ", HeaderParserResult::Unsupported(b"T")),
            (
                "mal formed: ",
                HeaderParserResult::Unsupported(b"mal formed"),
            ),
            (
                "MIME-version : ",
                HeaderParserResult::Supported(parse_mime_version),
            ),
        ];

        for input in inputs {
            let mut str = input.0.to_string();
            match parse_header_name(&MessageStream::new(unsafe { str.as_bytes_mut() })) {
                HeaderParserResult::Supported(f) => {
                    if let HeaderParserResult::Supported(val) = input.1 {
                        if f as usize == val as usize {
                            continue;
                        }
                    }
                }
                HeaderParserResult::Unsupported(name) => {
                    if let HeaderParserResult::Unsupported(val) = input.1 {
                        if name == val {
                            continue;
                        }
                    }
                }
                _ => panic!("Eof/Lf for '{:?}'", input.0),
            }
            panic!("Failed to parse '{:?}'", input.0)
        }
    }
}

static HDR_HASH: &[u8] = &[
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 15, 5, 0, 0, 30, 5, 5, 15, 62, 20, 15, 20, 5, 30, 25, 62, 0, 0, 0, 62, 62, 62, 62, 10,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62,
];

static HDR_FNCS: &[for<'x, 'y> fn(&mut dyn MessageField<'x>, &MessageStream<'x>)] = &[
    super::fields::parse_date,
    super::fields::parse_no_op,
    super::fields::parse_sender,
    super::fields::parse_subject,
    super::fields::parse_received,
    super::fields::parse_no_op,
    super::fields::parse_references,
    super::fields::parse_resent_date,
    super::fields::parse_cc,
    super::fields::parse_comments,
    super::fields::parse_resent_cc,
    super::fields::parse_content_id,
    super::fields::parse_return_path,
    super::fields::parse_resent_message_id,
    super::fields::parse_resent_sender,
    super::fields::parse_no_op,
    super::fields::parse_resent_bcc,
    super::fields::parse_no_op,
    super::fields::parse_list_id,
    super::fields::parse_bcc,
    super::fields::parse_list_post,
    super::fields::parse_list_owner,
    super::fields::parse_no_op,
    super::fields::parse_content_type,
    super::fields::parse_keywords,
    super::fields::parse_content_description,
    super::fields::parse_message_id,
    super::fields::parse_no_op,
    super::fields::parse_to,
    super::fields::parse_no_op,
    super::fields::parse_list_subscribe,
    super::fields::parse_content_transfer_encoding,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_reply_to,
    super::fields::parse_resent_to,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_list_archive,
    super::fields::parse_no_op,
    super::fields::parse_content_disposition,
    super::fields::parse_no_op,
    super::fields::parse_list_unsubscribe,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_list_help,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_mime_version,
    super::fields::parse_no_op,
    super::fields::parse_from,
    super::fields::parse_no_op,
    super::fields::parse_in_reply_to,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_no_op,
    super::fields::parse_resent_from,
];

static HDR_NAMES: &[&[u8]] = &[
    b"date",
    b"",
    b"sender",
    b"subject",
    b"received",
    b"",
    b"references",
    b"resent-date",
    b"cc",
    b"comments",
    b"resent-cc",
    b"content-id",
    b"return-path",
    b"resent-message-id",
    b"resent-sender",
    b"",
    b"resent-bcc",
    b"",
    b"list-id",
    b"bcc",
    b"list-post",
    b"list-owner",
    b"",
    b"content-type",
    b"keywords",
    b"content-description",
    b"message-id",
    b"",
    b"to",
    b"",
    b"list-subscribe",
    b"content-transfer-encoding",
    b"",
    b"",
    b"reply-to",
    b"resent-to",
    b"",
    b"",
    b"list-archive",
    b"",
    b"content-disposition",
    b"",
    b"list-unsubscribe",
    b"",
    b"",
    b"list-help",
    b"",
    b"",
    b"mime-version",
    b"",
    b"from",
    b"",
    b"in-reply-to",
    b"",
    b"",
    b"",
    b"",
    b"resent-from",
];
