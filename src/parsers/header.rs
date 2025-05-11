/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
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
                        _ => self.parse_raw(),
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
                    offset_field: offset_field as u32,
                    offset_start: from_offset as u32,
                    offset_end: self.offset() as u32,
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

        let mut header = [0u8; 30];

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
                        }

                        if let Some(header) = header.get_mut(token_len) {
                            *header = ch.to_ascii_lowercase();
                            token_len += 1;
                        }
                    }
                }
            }
        }

        if token_start != 0 {
            header_map(&header[..token_len])
                .unwrap_or_else(|| {
                    HeaderName::Other(String::from_utf8_lossy(
                        self.bytes(token_start - 1..token_end),
                    ))
                })
                .into()
        } else {
            None
        }
    }
}

impl<'x> HeaderName<'x> {
    /// Parse a header name
    pub fn parse(data: impl Into<Cow<'x, str>>) -> Option<HeaderName<'x>> {
        let data = data.into();

        if !data.is_empty() {
            let mut data_lc = String::with_capacity(data.len());
            for ch in data.chars() {
                match ch {
                    'A'..='Z' => data_lc.push(ch.to_ascii_lowercase()),
                    'a'..='z' | '0'..='9' | '-' | '_' => data_lc.push(ch),
                    _ => return None,
                }
            }
            header_map(data_lc.as_bytes())
                .unwrap_or(HeaderName::Other(data))
                .into()
        } else {
            None
        }
    }
}

fn header_map(name: &[u8]) -> Option<HeaderName<'static>> {
    hashify::tiny_map! {name,
    "arc-authentication-results" => HeaderName::ArcAuthenticationResults,
    "arc-seal" => HeaderName::ArcSeal,
    "arc-message-signature" => HeaderName::ArcMessageSignature,
    "bcc" => HeaderName::Bcc,
    "cc" => HeaderName::Cc,
    "comments" => HeaderName::Comments,
    "content-description" => HeaderName::ContentDescription,
    "content-disposition" => HeaderName::ContentDisposition,
    "content-id" => HeaderName::ContentId,
    "content-language" => HeaderName::ContentLanguage,
    "content-location" => HeaderName::ContentLocation,
    "content-transfer-encoding" => HeaderName::ContentTransferEncoding,
    "content-type" => HeaderName::ContentType,
    "date" => HeaderName::Date,
    "dkim-signature" => HeaderName::DkimSignature,
    "from" => HeaderName::From,
    "in-reply-to" => HeaderName::InReplyTo,
    "keywords" => HeaderName::Keywords,
    "list-archive" => HeaderName::ListArchive,
    "list-help" => HeaderName::ListHelp,
    "list-id" => HeaderName::ListId,
    "list-owner" => HeaderName::ListOwner,
    "list-post" => HeaderName::ListPost,
    "list-subscribe" => HeaderName::ListSubscribe,
    "list-unsubscribe" => HeaderName::ListUnsubscribe,
    "message-id" => HeaderName::MessageId,
    "mime-version" => HeaderName::MimeVersion,
    "received" => HeaderName::Received,
    "references" => HeaderName::References,
    "reply-to" => HeaderName::ReplyTo,
    "resent-bcc" => HeaderName::ResentBcc,
    "resent-cc" => HeaderName::ResentCc,
    "resent-date" => HeaderName::ResentDate,
    "resent-from" => HeaderName::ResentFrom,
    "resent-message-id" => HeaderName::ResentMessageId,
    "resent-sender" => HeaderName::ResentSender,
    "resent-to" => HeaderName::ResentTo,
    "return-path" => HeaderName::ReturnPath,
    "sender" => HeaderName::Sender,
    "subject" => HeaderName::Subject,
    "to" => HeaderName::To,
    }
}

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
