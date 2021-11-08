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

use std::collections::hash_map::Entry;

use crate::{HeaderName, HeaderValue, Headers};

use super::{
    fields::{
        address::parse_address,
        content_type::parse_content_type,
        date::parse_date,
        id::parse_id,
        list::parse_comma_separared,
        raw::{parse_and_ignore, parse_raw},
        unstructured::parse_unstructured,
    },
    message::MessageStream,
};

#[derive(Debug, PartialEq)]
pub enum HeaderParserResult<'x> {
    Supported(HeaderName),
    Unsupported(&'x [u8]),
    Lf,
    Eof,
}

pub fn parse_headers<'x>(headers: &mut Headers<'x>, stream: &mut MessageStream<'x>) -> bool {
    loop {
        let (bytes_read, result) = parse_header_name(&stream.data[stream.pos..]);
        stream.pos += bytes_read;

        match result {
            HeaderParserResult::Supported(name) => {
                let (is_many, parser) = HDR_PARSER[name as usize];

                let value = parser(stream);
                if !value.is_empty() {
                    if is_many {
                        match headers.entry(name) {
                            Entry::Occupied(mut e) => {
                                if let HeaderValue::Collection(col) = e.get_mut() {
                                    col.push(value);
                                } else {
                                    let old_value = e.remove();
                                    headers.insert(
                                        name,
                                        HeaderValue::Collection(vec![old_value, value]),
                                    );
                                }
                            }
                            Entry::Vacant(e) => {
                                e.insert(value);
                            }
                        }
                    } else {
                        headers.insert(name, value);
                    }
                }
            }
            HeaderParserResult::Unsupported(_name) => parse_and_ignore(stream),
            HeaderParserResult::Lf => return true,
            HeaderParserResult::Eof => return false,
        }
    }
}

pub fn parse_header_name(data: &[u8]) -> (usize, HeaderParserResult) {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut token_len: usize = 0;
    let mut token_hash: usize = 0;
    let mut last_ch: u8 = 0;
    let mut bytes_read: usize = 0;

    for ch in data.iter() {
        bytes_read += 1;

        match ch {
            b':' => {
                if token_start != 0 {
                    let field = &data[token_start - 1..token_end];

                    if (2..=25).contains(&token_len) {
                        token_hash +=
                            token_len + HDR_HASH[last_ch.to_ascii_lowercase() as usize] as usize;

                        if (4..=72).contains(&token_hash) {
                            let token_hash = token_hash - 4;

                            if field.eq_ignore_ascii_case(HDR_NAMES[token_hash]) {
                                return (
                                    bytes_read,
                                    HeaderParserResult::Supported(HDR_MAP[token_hash]),
                                );
                            }
                        }
                    }
                    return (bytes_read, HeaderParserResult::Unsupported(field));
                }
            }
            b'\n' => {
                return (bytes_read - 1, HeaderParserResult::Lf);
            }
            _ => {
                if !(*ch).is_ascii_whitespace() {
                    if token_start == 0 {
                        token_start = bytes_read;
                        token_end = token_start;
                    } else {
                        token_end = bytes_read;
                        last_ch = *ch;
                    }

                    if let 0 | 9 = token_len {
                        token_hash += HDR_HASH[(*ch).to_ascii_lowercase() as usize] as usize;
                    }
                    token_len += 1;
                }
            }
        }
    }

    (bytes_read, HeaderParserResult::Eof)
}

/*fn set_unsupported(&mut self, stream: &mut MessageStream<'x>, name: &'x [u8]) {
    if let Ok(name) = std::str::from_utf8(name) {
        let value = parse_unstructured(stream);
        if !value.is_empty() {
            match self.others.entry(name) {
                Entry::Occupied(mut e) => {
                    if let HeaderValue::Collection(col) = e.get_mut() {
                        col.push(value);
                    } else {
                        let old_value = e.remove();
                        self.others.insert(name,HeaderValue::Collection(vec![old_value, value]));
                    }
                },
                Entry::Vacant(e) => {e.insert(value);},
            }
        }
    }
}*/

#[cfg(test)]
mod tests {
    use crate::{parsers::header::parse_header_name, HeaderName};

    use super::HeaderParserResult;

    #[test]
    fn header_name_parse() {
        let inputs = [
            ("From: ", HeaderParserResult::Supported(HeaderName::From)),
            (
                "receiVED: ",
                HeaderParserResult::Supported(HeaderName::Received),
            ),
            (
                " subject   : ",
                HeaderParserResult::Supported(HeaderName::Subject),
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
                HeaderParserResult::Supported(HeaderName::MimeVersion),
            ),
        ];

        for input in inputs {
            let str = input.0.to_string();
            let (_, result) = parse_header_name(str.as_bytes());
            assert_eq!(input.1, result, "Failed to parse '{:?}'", input.0);
        }
    }
}

#[allow(clippy::type_complexity)]
static HDR_PARSER: &[(
    bool,
    for<'x, 'y> fn(&mut MessageStream<'x>) -> HeaderValue<'x>,
)] = &[
    (false, parse_unstructured),    // Subject = 0,
    (false, parse_address),         // From = 1,
    (false, parse_address),         // To = 2,
    (false, parse_address),         // Cc = 3,
    (false, parse_date),            // Date = 4,
    (false, parse_address),         // Bcc = 5,
    (false, parse_address),         // ReplyTo = 6,
    (false, parse_address),         // Sender = 7,
    (true, parse_unstructured),     // Comments = 8,
    (false, parse_address),         // InReplyTo = 9,
    (true, parse_comma_separared),  // Keywords = 10,
    (true, parse_raw),              // Received = 11,
    (false, parse_id),              // MessageId = 12,
    (true, parse_id),               // References = 13, (RFC 5322 recommends One)
    (false, parse_id),              // ReturnPath = 14,
    (false, parse_raw),             // MimeVersion = 15,
    (false, parse_unstructured),    // ContentDescription = 16,
    (false, parse_id),              // ContentId = 17,
    (false, parse_comma_separared), // ContentLanguage = 18
    (false, parse_unstructured),    // ContentLocation = 19
    (false, parse_unstructured),    // ContentTransferEncoding = 20,
    (false, parse_content_type),    // ContentType = 21,
    (false, parse_content_type),    // ContentDisposition = 22,
    (true, parse_address),          // ResentTo = 23,
    (true, parse_address),          // ResentFrom = 24,
    (true, parse_address),          // ResentBcc = 25,
    (true, parse_address),          // ResentCc = 26,
    (true, parse_address),          // ResentSender = 27,
    (true, parse_date),             // ResentDate = 28,
    (true, parse_id),               // ResentMessageId = 29,
    (false, parse_address),         // ListArchive = 30,
    (false, parse_address),         // ListHelp = 31,
    (false, parse_address),         // ListId = 32,
    (false, parse_address),         // ListOwner = 33,
    (false, parse_address),         // ListPost = 34,
    (false, parse_address),         // ListSubscribe = 35,
    (false, parse_address),         // ListUnsubscribe = 36,
    (false, parse_raw),             // Unsupported = 37,
];

static HDR_HASH: &[u8] = &[
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 0, 20, 5, 0, 0, 25, 0, 5, 20, 73, 25, 25, 30, 10, 10, 5, 73, 0, 0, 15, 73, 73, 73, 73, 20,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73,
];

static HDR_MAP: &[HeaderName] = &[
    HeaderName::Date,
    HeaderName::Unsupported,
    HeaderName::Sender,
    HeaderName::Unsupported,
    HeaderName::Received,
    HeaderName::Unsupported,
    HeaderName::References,
    HeaderName::Unsupported,
    HeaderName::Cc,
    HeaderName::Comments,
    HeaderName::ResentCc,
    HeaderName::ContentId,
    HeaderName::Unsupported,
    HeaderName::ResentMessageId,
    HeaderName::ReplyTo,
    HeaderName::ResentTo,
    HeaderName::ResentBcc,
    HeaderName::ContentLanguage,
    HeaderName::Subject,
    HeaderName::ResentSender,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::ResentDate,
    HeaderName::To,
    HeaderName::Bcc,
    HeaderName::Unsupported,
    HeaderName::ContentTransferEncoding,
    HeaderName::ReturnPath,
    HeaderName::ListId,
    HeaderName::Keywords,
    HeaderName::ContentDescription,
    HeaderName::ListOwner,
    HeaderName::Unsupported,
    HeaderName::ContentType,
    HeaderName::Unsupported,
    HeaderName::ListHelp,
    HeaderName::MessageId,
    HeaderName::ContentLocation,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::ListSubscribe,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::ListPost,
    HeaderName::Unsupported,
    HeaderName::ResentFrom,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::ContentDisposition,
    HeaderName::Unsupported,
    HeaderName::InReplyTo,
    HeaderName::ListArchive,
    HeaderName::Unsupported,
    HeaderName::From,
    HeaderName::Unsupported,
    HeaderName::ListUnsubscribe,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::Unsupported,
    HeaderName::MimeVersion,
];

static HDR_NAMES: &[&[u8]] = &[
    b"date",
    b"",
    b"sender",
    b"",
    b"received",
    b"",
    b"references",
    b"",
    b"cc",
    b"comments",
    b"resent-cc",
    b"content-id",
    b"",
    b"resent-message-id",
    b"reply-to",
    b"resent-to",
    b"resent-bcc",
    b"content-language",
    b"subject",
    b"resent-sender",
    b"",
    b"",
    b"resent-date",
    b"to",
    b"bcc",
    b"",
    b"content-transfer-encoding",
    b"return-path",
    b"list-id",
    b"keywords",
    b"content-description",
    b"list-owner",
    b"",
    b"content-type",
    b"",
    b"list-help",
    b"message-id",
    b"content-location",
    b"",
    b"",
    b"list-subscribe",
    b"",
    b"",
    b"",
    b"",
    b"list-post",
    b"",
    b"resent-from",
    b"",
    b"",
    b"content-disposition",
    b"",
    b"in-reply-to",
    b"list-archive",
    b"",
    b"from",
    b"",
    b"list-unsubscribe",
    b"",
    b"",
    b"",
    b"",
    b"",
    b"",
    b"",
    b"",
    b"",
    b"",
    b"mime-version",
];
