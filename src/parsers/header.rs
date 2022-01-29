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

use std::{borrow::Cow, collections::hash_map::Entry};

use crate::{HeaderName, HeaderOffset, HeaderValue, RawHeaders, RfcHeader, RfcHeaders};

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
    Rfc(RfcHeader),
    Other(Cow<'x, str>),
    Lf,
    Eof,
}

pub fn parse_headers<'x>(
    headers_rfc: &mut RfcHeaders<'x>,
    headers_raw: &mut RawHeaders<'x>,
    stream: &mut MessageStream<'x>,
) -> bool {
    loop {
        let (bytes_read, result) = parse_header_name(&stream.data[stream.pos..]);
        stream.pos += bytes_read;

        match result {
            HeaderParserResult::Rfc(name) => {
                let (_, parser) = HDR_PARSER[name as usize];

                let from_offset = stream.pos;
                let value = parser(stream);
                headers_raw
                    .entry(HeaderName::Rfc(name))
                    .or_insert_with(Vec::new)
                    .push(HeaderOffset {
                        start: from_offset,
                        end: stream.pos,
                    });

                if !value.is_empty() {
                    match headers_rfc.entry(name) {
                        Entry::Occupied(mut e) => {
                            if let HeaderValue::Collection(col) = e.get_mut() {
                                col.push(value);
                            } else {
                                let old_value = e.remove();
                                headers_rfc
                                    .insert(name, HeaderValue::Collection(vec![old_value, value]));
                            }
                        }
                        Entry::Vacant(e) => {
                            e.insert(value);
                        }
                    }
                }
            }
            HeaderParserResult::Other(name) => {
                let from_offset = stream.pos;
                parse_and_ignore(stream);
                headers_raw
                    .entry(HeaderName::Other(name))
                    .or_insert_with(Vec::new)
                    .push(HeaderOffset {
                        start: from_offset,
                        end: stream.pos,
                    });
            }
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
                    break;
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

    if token_start != 0 {
        let field = &data[token_start - 1..token_end];

        if (2..=25).contains(&token_len) {
            token_hash += token_len + HDR_HASH[last_ch.to_ascii_lowercase() as usize] as usize;

            if (4..=72).contains(&token_hash) {
                let token_hash = token_hash - 4;

                if field.eq_ignore_ascii_case(HDR_NAMES[token_hash]) {
                    return (bytes_read, HeaderParserResult::Rfc(HDR_MAP[token_hash]));
                }
            }
        }
        return (
            bytes_read,
            HeaderParserResult::Other(String::from_utf8_lossy(field)),
        );
    } else {
        (bytes_read, HeaderParserResult::Eof)
    }
}

impl From<RfcHeader> for u8 {
    fn from(name: RfcHeader) -> Self {
        name as u8
    }
}

#[cfg(test)]
mod tests {
    use crate::{parsers::header::parse_header_name, RfcHeader};

    use super::HeaderParserResult;

    #[test]
    fn header_name_parse() {
        let inputs = [
            ("From: ", HeaderParserResult::Rfc(RfcHeader::From)),
            ("receiVED: ", HeaderParserResult::Rfc(RfcHeader::Received)),
            (" subject   : ", HeaderParserResult::Rfc(RfcHeader::Subject)),
            (
                "X-Custom-Field : ",
                HeaderParserResult::Other("X-Custom-Field".into()),
            ),
            (" T : ", HeaderParserResult::Other("T".into())),
            (
                "mal formed: ",
                HeaderParserResult::Other("mal formed".into()),
            ),
            (
                "MIME-version : ",
                HeaderParserResult::Rfc(RfcHeader::MimeVersion),
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
    (false, parse_id),              // InReplyTo = 9,
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
    (true, parse_raw),              // Other = 37,
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

static HDR_MAP: &[RfcHeader] = &[
    RfcHeader::Date,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::Sender,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::Received,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::References,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::Cc,
    RfcHeader::Comments,
    RfcHeader::ResentCc,
    RfcHeader::ContentId,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ResentMessageId,
    RfcHeader::ReplyTo,
    RfcHeader::ResentTo,
    RfcHeader::ResentBcc,
    RfcHeader::ContentLanguage,
    RfcHeader::Subject,
    RfcHeader::ResentSender,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ResentDate,
    RfcHeader::To,
    RfcHeader::Bcc,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ContentTransferEncoding,
    RfcHeader::ReturnPath,
    RfcHeader::ListId,
    RfcHeader::Keywords,
    RfcHeader::ContentDescription,
    RfcHeader::ListOwner,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ContentType,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ListHelp,
    RfcHeader::MessageId,
    RfcHeader::ContentLocation,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ListSubscribe,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ListPost,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ResentFrom,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ContentDisposition,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::InReplyTo,
    RfcHeader::ListArchive,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::From,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::ListUnsubscribe,
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion, // Invalid
    RfcHeader::MimeVersion,
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
