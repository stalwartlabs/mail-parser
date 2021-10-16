use std::{borrow::Cow, collections::HashMap};

use crate::parsers::message_stream::MessageStream;

use super::fields::date::DateTime;

pub struct Message<'x> {
    subject: Option<Cow<'x, str>>,
}

#[derive(PartialEq, Debug)]
pub struct NamedValue<'x> {
    name: Cow<'x, str>,
    subname: Option<Cow<'x, str>>,
    value: HeaderValue<'x>,
}

impl<'x> NamedValue<'x> {
    pub fn new(
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

pub trait MessageHeader<'x> {
    fn set_subject(&mut self, stream: &'x MessageStream);
    fn set_from(&mut self, stream: &'x MessageStream);
}

enum HeaderParserResult<'x, T> {
    Supported(HeaderParserFnc<'x, T>),
    Unsupported(&'x [u8]),
    Eof,
}

type HeaderParserFnc<'x, T> = fn(&'x mut T, &MessageStream<'x>);

pub fn parse_headers<'x>(stream: &'x MessageStream) {}

fn parse_header_name<'x, T>(stream: &'x MessageStream) -> HeaderParserResult<'x, T> {
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

                        if (4..=62).contains(&token_hash) {
                            let token_hash = token_hash - 4;

                            if field.eq_ignore_ascii_case(unsafe {
                                HDR_NAMES.get_unchecked(token_hash)
                            }) {
                                println!("Supported '{}'", std::str::from_utf8(field).unwrap());
                                return HeaderParserResult::Eof;
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
    use crate::parsers::{header::parse_header_name, message_stream::MessageStream};

    #[test]
    fn header_name_parse() {
        let inputs = [
            ("From: ", "from"),
            ("\n\n \nreceiVED: ", "received"),
            (" subject   : ", "subject"),
            ("X-Custom-Field : ", "x-custom-field"),
            (" T : ", "t"),
            ("mal formed: ", "mal formed"),
            ("MIME-version : ", "x-custom-field"),
        ];

        for input in inputs {
            //unreachable!();
            /*match parse_header_name(&MessageStream::new(input.0.as_bytes())) {
                super::HeaderParserResult::Supported(_) => (),
                super::HeaderParserResult::Unsupported(_) => (),
                super::HeaderParserResult::Eof => (),
            }*/
        }
    }
}

static HDR_HASH: &[u8] = &[
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 20, 5, 0, 0, 0, 0, 5, 5, 63, 15, 15, 25, 20, 10, 0, 63, 0, 0, 15, 63, 63, 63, 63, 20,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
];

/*static HDR_FNCS: &[for<'r, 's> fn(&'r mut T, &MessageStream<'s>)] = &[
    MessageHeader::set_subject,
    MessageHeader::set_from,
];*/

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
    b"",
    b"subject",
    b"keywords",
    b"list-help",
    b"list-owner",
    b"resent-date",
    b"to",
    b"bcc",
    b"from",
    b"content-transfer-encoding",
    b"return-path",
    b"list-archive",
    b"resent-sender",
    b"list-subscribe",
    b"message-id",
    b"",
    b"content-type",
    b"",
    b"list-post",
    b"",
    b"in-reply-to",
    b"",
    b"",
    b"content-description",
    b"",
    b"resent-from",
    b"",
    b"",
    b"content-disposition",
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
