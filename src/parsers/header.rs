use std::{borrow::Cow, collections::HashMap};

use crate::parsers::message_stream::MessageStream;

use super::fields::{date::DateTime, parse_unsupported};

#[derive(PartialEq, Debug)]
pub struct NamedValue<'x> {
    name: Cow<'x, str>,
    subname: Option<Cow<'x, str>>,
    value: HeaderValue<'x>,
}

impl<'x> NamedValue<'x> {
    pub fn boxed(
        name: Cow<'x, str>,
        subname: Option<Cow<'x, str>>,
        value: HeaderValue<'x>,
    ) -> HeaderValue<'x> {
        HeaderValue::NamedValue(Box::new(NamedValue {
            name,
            subname,
            value,
        }))
    }
}

#[derive(PartialEq, Debug)]
pub enum HeaderValue<'x> {
    Empty,
    DateTime(Box<DateTime>),
    String(Cow<'x, str>),
    Array(Vec<HeaderValue<'x>>),
    Map(HashMap<Cow<'x, str>, Cow<'x, str>>),
    NamedValue(Box<NamedValue<'x>>),
}

impl<'x> Default for HeaderValue<'x> {
    fn default() -> Self {
        HeaderValue::Empty
    }
}

#[derive(PartialEq, Debug, Default)]
pub struct Header<'x> {
    pub bcc: HeaderValue<'x>,
    pub cc: HeaderValue<'x>,
    pub comments: HeaderValue<'x>,
    pub content_description: HeaderValue<'x>,
    pub content_disposition: HeaderValue<'x>,
    pub content_id: HeaderValue<'x>,
    pub content_transfer_encoding: HeaderValue<'x>,
    pub content_type: HeaderValue<'x>,
    pub date: HeaderValue<'x>,
    pub from: HeaderValue<'x>,
    pub in_reply_to: HeaderValue<'x>,
    pub keywords: HeaderValue<'x>,
    pub list_archive: HeaderValue<'x>,
    pub list_help: HeaderValue<'x>,
    pub list_id: HeaderValue<'x>,
    pub list_owner: HeaderValue<'x>,
    pub list_post: HeaderValue<'x>,
    pub list_subscribe: HeaderValue<'x>,
    pub list_unsubscribe: HeaderValue<'x>,
    pub message_id: HeaderValue<'x>,
    pub mime_version: HeaderValue<'x>,
    pub received: HeaderValue<'x>,
    pub references: HeaderValue<'x>,
    pub reply_to: HeaderValue<'x>,
    pub resent_bcc: HeaderValue<'x>,
    pub resent_cc: HeaderValue<'x>,
    pub resent_date: HeaderValue<'x>,
    pub resent_from: HeaderValue<'x>,
    pub resent_message_id: HeaderValue<'x>,
    pub resent_sender: HeaderValue<'x>,
    pub resent_to: HeaderValue<'x>,
    pub return_path: HeaderValue<'x>,
    pub sender: HeaderValue<'x>,
    pub subject: HeaderValue<'x>,
    pub to: HeaderValue<'x>,
    pub others: HashMap<&'x str, HeaderValue<'x>>,
}

impl<'x> Header<'x> {
    pub fn new() -> Header<'x> {
        Header {
            ..Default::default()
        }
    }
}

enum HeaderParserResult<'x> {
    Supported(fn(&mut Header<'x>, &'x MessageStream<'x>)),
    Unsupported(&'x [u8]),
    Eof,
}

pub fn parse_headers<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    loop {
        match parse_header_name(stream) {
            HeaderParserResult::Supported(fnc) => fnc(header, stream),
            HeaderParserResult::Unsupported(name) => parse_unsupported(header, stream, name),
            HeaderParserResult::Eof => return,
        }
    }
}

fn parse_header_name<'x>(stream: &'x MessageStream) -> HeaderParserResult<'x> {
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
                if token_start == 0 {
                    break;
                }
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
            match parse_header_name(&MessageStream::new(input.0.as_bytes())) {
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
                HeaderParserResult::Eof => panic!("EOF for '{}'", input.0.escape_debug()),
            }
            panic!("Failed to parse '{}'", input.0.escape_debug())
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

static HDR_FNCS: &[for<'x, 'y> fn(&'y mut Header<'x>, &'x MessageStream<'x>)] = &[
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
