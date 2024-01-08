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

use std::borrow::Cow;

use crate::{Header, HeaderName, MessageParser};

use super::MessageStream;

impl<'x> MessageStream<'x> {
    pub fn parse_headers(&mut self, conf: &MessageParser, headers: &mut Vec<Header<'x>>) -> bool {
        loop {
            loop {
                match self.peek() {
                    Some(b'\n') => {
                        self.next();
                        return true;
                    }
                    None => return false,
                    Some(ch) if !ch.is_ascii_whitespace() => {
                        break;
                    }
                    _ => {
                        self.next();
                    }
                }
            }

            let offset_field = self.offset();

            if let Some(header_name) = self.parse_header_name() {
                let from_offset = self.offset();
                let value = if conf.header_map.is_empty() {
                    match &header_name {
                        HeaderName::Subject
                        | HeaderName::Comments
                        | HeaderName::ContentDescription
                        | HeaderName::ContentLocation
                        | HeaderName::ContentTransferEncoding => self.parse_unstructured(),
                        HeaderName::From
                        | HeaderName::To
                        | HeaderName::Cc
                        | HeaderName::Bcc
                        | HeaderName::ReplyTo
                        | HeaderName::Sender
                        | HeaderName::ResentTo
                        | HeaderName::ResentFrom
                        | HeaderName::ResentBcc
                        | HeaderName::ResentCc
                        | HeaderName::ResentSender
                        | HeaderName::ListArchive
                        | HeaderName::ListHelp
                        | HeaderName::ListId
                        | HeaderName::ListOwner
                        | HeaderName::ListPost
                        | HeaderName::ListSubscribe
                        | HeaderName::ListUnsubscribe => self.parse_address(),
                        HeaderName::Date | HeaderName::ResentDate => self.parse_date(),
                        HeaderName::MessageId
                        | HeaderName::References
                        | HeaderName::InReplyTo
                        | HeaderName::ReturnPath
                        | HeaderName::ContentId
                        | HeaderName::ResentMessageId => self.parse_id(),
                        HeaderName::Keywords | HeaderName::ContentLanguage => {
                            self.parse_comma_separared()
                        }
                        HeaderName::Received => self.parse_received(),
                        HeaderName::MimeVersion => self.parse_raw(),
                        HeaderName::ContentType | HeaderName::ContentDisposition => {
                            self.parse_content_type()
                        }
                        HeaderName::Other(_) => self.parse_raw(),
                    }
                } else {
                    (conf
                        .header_map
                        .get(&header_name)
                        .unwrap_or(&conf.def_hdr_parse_fnc))(self)
                };

                headers.push(Header {
                    name: header_name,
                    value,
                    offset_field,
                    offset_start: from_offset,
                    offset_end: self.offset(),
                });
            } else if self.is_eof() {
                return false;
            }
        }
    }

    pub fn parse_header_name(&mut self) -> Option<HeaderName<'x>> {
        let mut token_start: usize = 0;
        let mut token_end: usize = 0;
        let mut token_len: usize = 0;
        let mut token_hash: usize = 0;
        let mut last_ch: u8 = 0;

        while let Some(&ch) = self.next() {
            match ch {
                b':' => {
                    if token_start != 0 {
                        break;
                    }
                }
                b'\n' => {
                    return None;
                }
                _ => {
                    if !ch.is_ascii_whitespace() {
                        if token_start == 0 {
                            token_start = self.offset();
                            token_end = token_start;
                        } else {
                            token_end = self.offset();
                            last_ch = ch;
                        }

                        if let 0 | 9 = token_len {
                            token_hash += {
                                #[cfg(feature = "ludicrous_mode")]
                                unsafe {
                                    *HDR_HASH.get_unchecked(ch.to_ascii_lowercase() as usize)
                                }

                                #[cfg(not(feature = "ludicrous_mode"))]
                                HDR_HASH[ch.to_ascii_lowercase() as usize]
                            } as usize;
                        }
                        token_len += 1;
                    }
                }
            }
        }

        if token_start != 0 {
            let field = self.bytes(token_start - 1..token_end);

            if (2..=25).contains(&token_len) {
                token_hash += token_len + {
                    #[cfg(feature = "ludicrous_mode")]
                    unsafe {
                        *HDR_HASH.get_unchecked(last_ch.to_ascii_lowercase() as usize)
                    }

                    #[cfg(not(feature = "ludicrous_mode"))]
                    HDR_HASH[last_ch.to_ascii_lowercase() as usize]
                } as usize;

                if (4..=72).contains(&token_hash) {
                    let token_hash = token_hash - 4;

                    if field.eq_ignore_ascii_case(HDR_NAMES[token_hash]) {
                        return Some(HDR_MAP[token_hash].clone());
                    }
                }
            }
            Some(HeaderName::Other(String::from_utf8_lossy(field)))
        } else {
            None
        }
    }
}

impl<'x> HeaderName<'x> {
    /// Parse a header name
    pub fn parse(data: impl Into<Cow<'x, str>>) -> Option<HeaderName<'x>> {
        let mut token_hash: usize = 0;
        let mut last_ch: u8 = 0;
        let data = data.into();
        let data_ = data.as_bytes();

        for (pos, &ch) in data_.iter().enumerate() {
            if ch.is_ascii_alphanumeric() || [b'_', b'-'].contains(&ch) {
                if let 0 | 9 = pos {
                    token_hash += {
                        #[cfg(feature = "ludicrous_mode")]
                        unsafe {
                            *HDR_HASH.get_unchecked(ch.to_ascii_lowercase() as usize)
                        }

                        #[cfg(not(feature = "ludicrous_mode"))]
                        HDR_HASH[ch.to_ascii_lowercase() as usize]
                    } as usize;
                }
                last_ch = ch;
            } else {
                return None;
            }
        }

        if (2..=25).contains(&data.len()) {
            token_hash += data.len() + {
                #[cfg(feature = "ludicrous_mode")]
                unsafe {
                    *HDR_HASH.get_unchecked(last_ch.to_ascii_lowercase() as usize)
                }

                #[cfg(not(feature = "ludicrous_mode"))]
                HDR_HASH[last_ch.to_ascii_lowercase() as usize]
            } as usize;

            if (4..=72).contains(&token_hash) {
                let token_hash = token_hash - 4;

                if data_.eq_ignore_ascii_case(HDR_NAMES[token_hash]) {
                    return HDR_MAP[token_hash].clone().into();
                }
            }
        }

        if !data.is_empty() {
            HeaderName::Other(data).into()
        } else {
            None
        }
    }
}

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
    HeaderName::MimeVersion, // Invalid
    HeaderName::Sender,
    HeaderName::MimeVersion, // Invalid
    HeaderName::Received,
    HeaderName::MimeVersion, // Invalid
    HeaderName::References,
    HeaderName::MimeVersion, // Invalid
    HeaderName::Cc,
    HeaderName::Comments,
    HeaderName::ResentCc,
    HeaderName::ContentId,
    HeaderName::MimeVersion, // Invalid
    HeaderName::ResentMessageId,
    HeaderName::ReplyTo,
    HeaderName::ResentTo,
    HeaderName::ResentBcc,
    HeaderName::ContentLanguage,
    HeaderName::Subject,
    HeaderName::ResentSender,
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::ResentDate,
    HeaderName::To,
    HeaderName::Bcc,
    HeaderName::MimeVersion, // Invalid
    HeaderName::ContentTransferEncoding,
    HeaderName::ReturnPath,
    HeaderName::ListId,
    HeaderName::Keywords,
    HeaderName::ContentDescription,
    HeaderName::ListOwner,
    HeaderName::MimeVersion, // Invalid
    HeaderName::ContentType,
    HeaderName::MimeVersion, // Invalid
    HeaderName::ListHelp,
    HeaderName::MessageId,
    HeaderName::ContentLocation,
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::ListSubscribe,
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::ListPost,
    HeaderName::MimeVersion, // Invalid
    HeaderName::ResentFrom,
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::ContentDisposition,
    HeaderName::MimeVersion, // Invalid
    HeaderName::InReplyTo,
    HeaderName::ListArchive,
    HeaderName::MimeVersion, // Invalid
    HeaderName::From,
    HeaderName::MimeVersion, // Invalid
    HeaderName::ListUnsubscribe,
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
    HeaderName::MimeVersion, // Invalid
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

#[cfg(test)]
mod tests {
    use crate::{parsers::MessageStream, HeaderName};

    #[test]
    fn header_name_parse() {
        let inputs = [
            ("From: ", HeaderName::From),
            ("receiVED: ", HeaderName::Received),
            (" subject   : ", HeaderName::Subject),
            (
                "X-Custom-Field : ",
                HeaderName::Other("X-Custom-Field".into()),
            ),
            (" T : ", HeaderName::Other("T".into())),
            ("mal formed: ", HeaderName::Other("mal formed".into())),
            ("MIME-version : ", HeaderName::MimeVersion),
        ];

        for (input, expected_result) in inputs {
            assert_eq!(
                expected_result,
                MessageStream::new(input.as_bytes())
                    .parse_header_name()
                    .unwrap(),
                "Failed to parse '{input:?}'",
            );
        }
    }
}
