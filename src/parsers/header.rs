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

use crate::{Header, HeaderName, RfcHeader};

use super::MessageStream;

impl<'x> MessageStream<'x> {
    pub fn parse_headers(&mut self, headers: &mut Vec<Header<'x>>) -> bool {
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
                let value = if let HeaderName::Rfc(rfc_name) = &header_name {
                    match rfc_name {
                        RfcHeader::Subject
                        | RfcHeader::Comments
                        | RfcHeader::ContentDescription
                        | RfcHeader::ContentLocation
                        | RfcHeader::ContentTransferEncoding => self.parse_unstructured(),
                        RfcHeader::From
                        | RfcHeader::To
                        | RfcHeader::Cc
                        | RfcHeader::Bcc
                        | RfcHeader::ReplyTo
                        | RfcHeader::Sender
                        | RfcHeader::ResentTo
                        | RfcHeader::ResentFrom
                        | RfcHeader::ResentBcc
                        | RfcHeader::ResentCc
                        | RfcHeader::ResentSender
                        | RfcHeader::ListArchive
                        | RfcHeader::ListHelp
                        | RfcHeader::ListId
                        | RfcHeader::ListOwner
                        | RfcHeader::ListPost
                        | RfcHeader::ListSubscribe
                        | RfcHeader::ListUnsubscribe => self.parse_address(),
                        RfcHeader::Date | RfcHeader::ResentDate => self.parse_date(),
                        RfcHeader::MessageId
                        | RfcHeader::References
                        | RfcHeader::InReplyTo
                        | RfcHeader::ReturnPath
                        | RfcHeader::ContentId
                        | RfcHeader::ResentMessageId => self.parse_id(),
                        RfcHeader::Keywords | RfcHeader::ContentLanguage => {
                            self.parse_comma_separared()
                        }
                        RfcHeader::Received | RfcHeader::MimeVersion => self.parse_raw(),
                        RfcHeader::ContentType | RfcHeader::ContentDisposition => {
                            self.parse_content_type()
                        }
                    }
                } else {
                    self.parse_raw()
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
                        return Some(HeaderName::Rfc(HDR_MAP[token_hash]));
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
                    return HeaderName::Rfc(HDR_MAP[token_hash]).into();
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

impl From<RfcHeader> for u8 {
    fn from(name: RfcHeader) -> Self {
        name as u8
    }
}

#[cfg(test)]
mod tests {
    use crate::{parsers::MessageStream, HeaderName, RfcHeader};

    #[test]
    fn header_name_parse() {
        let inputs = [
            ("From: ", HeaderName::Rfc(RfcHeader::From)),
            ("receiVED: ", HeaderName::Rfc(RfcHeader::Received)),
            (" subject   : ", HeaderName::Rfc(RfcHeader::Subject)),
            (
                "X-Custom-Field : ",
                HeaderName::Other("X-Custom-Field".into()),
            ),
            (" T : ", HeaderName::Other("T".into())),
            ("mal formed: ", HeaderName::Other("mal formed".into())),
            ("MIME-version : ", HeaderName::Rfc(RfcHeader::MimeVersion)),
        ];

        for (input, expected_result) in inputs {
            assert_eq!(
                expected_result,
                MessageStream::new(input.as_bytes())
                    .parse_header_name()
                    .unwrap(),
                "Failed to parse '{:?}'",
                input
            );
        }
    }
}

/*
type ParserFnc<'x> = fn(&mut MessageStream<'x>) -> HeaderValue<'x>;

static HDR_PARSER: &[(bool, for<'x> fn(&mut MessageStream<'x>) -> HeaderValue<'x>] = &[
    (false, MessageStream::parse_unstructured), // Subject = 0,
    (false, MessageStream::parse_address),      // From = 1,
    (false, MessageStream::parse_address),      // To = 2,
    (false, MessageStream::parse_address),      // Cc = 3,
    (false, MessageStream::parse_date),         // Date = 4,
    (false, MessageStream::parse_address),      // Bcc = 5,
    (false, MessageStream::parse_address),      // ReplyTo = 6,
    (false, MessageStream::parse_address),      // Sender = 7,
    (true, MessageStream::parse_unstructured),  // Comments = 8,
    (false, MessageStream::parse_id),           // InReplyTo = 9,
    (true, MessageStream::parse_comma_separared), // Keywords = 10,
    (true, MessageStream::parse_raw),           // Received = 11,
    (false, MessageStream::parse_id),           // MessageId = 12,
    (true, MessageStream::parse_id),            // References = 13, (RFC 5322 recommends One)
    (false, MessageStream::parse_id),           // ReturnPath = 14,
    (false, MessageStream::parse_raw),          // MimeVersion = 15,
    (false, MessageStream::parse_unstructured), // ContentDescription = 16,
    (false, MessageStream::parse_id),           // ContentId = 17,
    (false, MessageStream::parse_comma_separared), // ContentLanguage = 18
    (false, MessageStream::parse_unstructured), // ContentLocation = 19
    (false, MessageStream::parse_unstructured), // ContentTransferEncoding = 20,
    (false, MessageStream::parse_content_type), // ContentType = 21,
    (false, MessageStream::parse_content_type), // ContentDisposition = 22,
    (true, MessageStream::parse_address),       // ResentTo = 23,
    (true, MessageStream::parse_address),       // ResentFrom = 24,
    (true, MessageStream::parse_address),       // ResentBcc = 25,
    (true, MessageStream::parse_address),       // ResentCc = 26,
    (true, MessageStream::parse_address),       // ResentSender = 27,
    (true, MessageStream::parse_date),          // ResentDate = 28,
    (true, MessageStream::parse_id),            // ResentMessageId = 29,
    (false, MessageStream::parse_address),      // ListArchive = 30,
    (false, MessageStream::parse_address),      // ListHelp = 31,
    (false, MessageStream::parse_address),      // ListId = 32,
    (false, MessageStream::parse_address),      // ListOwner = 33,
    (false, MessageStream::parse_address),      // ListPost = 34,
    (false, MessageStream::parse_address),      // ListSubscribe = 35,
    (false, MessageStream::parse_address),      // ListUnsubscribe = 36,
    (true, MessageStream::parse_raw),           // Other = 37,
];*/

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
