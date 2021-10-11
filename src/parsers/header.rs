use std::borrow::Cow;

use crate::parsers::fields::address::{Address, AddressList};
use crate::parsers::message_stream::MessageStream;

use super::fields::unstructured::parse_unstructured;

pub enum Header<'x> {
    /*Sender(Address),
    From(Address),
    To(AddressList),
    Cc(AddressList),
    Bcc(AddressList),*/
    Subject(Cow<'x, str>),
    Other((&'x str, &'x str)),
}

impl<'x> Header<'x> {
    pub fn parse(stream: &'x MessageStream) -> Option<Header<'x>> {
        while let Some(name) = Header::parse_hdr_name(stream) {
            let mut hash = name.len();

            if !(2..=25).contains(&hash) {
                return None;
            }

            hash += unsafe {
                *HDR_HASH.get_unchecked(name.get_unchecked(0).to_ascii_lowercase() as usize)
                    as usize
                    + *HDR_HASH
                        .get_unchecked(name.get_unchecked(hash - 1).to_ascii_lowercase() as usize)
                        as usize
            };

            if !(2..=44).contains(&hash) {
                return None;
            }

            match hash - 2 {
                0 if name.eq_ignore_ascii_case(b"cc") => {
                    println!("Got 'cc'");
                }
                1 if name.eq_ignore_ascii_case(b"bcc") => {
                    println!("Got 'bcc'");
                }
                7 if name.eq_ignore_ascii_case(b"resent-cc") => {
                    println!("Got 'resent-cc'");
                }
                8 if name.eq_ignore_ascii_case(b"resent-bcc") => {
                    println!("Got 'resent-bcc'");
                }
                9 if name.eq_ignore_ascii_case(b"resent-date") => {
                    println!("Got 'resent-date'");
                }
                10 if name.eq_ignore_ascii_case(b"content-type") => {
                    println!("Got 'content-type'");
                }
                11 if name.eq_ignore_ascii_case(b"resent-sender") => {
                    println!("Got 'resent-sender'");
                }
                12 if name.eq_ignore_ascii_case(b"date") => {
                    println!("Got 'date'");
                }
                13 if name.eq_ignore_ascii_case(b"list-owner") => {
                    println!("Got 'list-owner'");
                }
                14 if name.eq_ignore_ascii_case(b"resent-from") => {
                    println!("Got 'resent-from'");
                }
                15 if name.eq_ignore_ascii_case(b"list-archive") => {
                    println!("Got 'list-archive'");
                }
                16 if name.eq_ignore_ascii_case(b"received") => {
                    println!("Got 'received'");
                }
                17 if name.eq_ignore_ascii_case(b"list-subscribe") => {
                    println!("Got 'list-subscribe'");
                }
                18 if name.eq_ignore_ascii_case(b"content-id") => {
                    println!("Got 'content-id'");
                }
                19 if name.eq_ignore_ascii_case(b"list-unsubscribe") => {
                    println!("Got 'list-unsubscribe'");
                }
                22 if name.eq_ignore_ascii_case(b"list-post") => {
                    println!("Got 'list-post'");
                }
                23 if name.eq_ignore_ascii_case(b"message-id") => {
                    println!("Got 'message-id'");
                }
                24 if name.eq_ignore_ascii_case(b"sender") => {
                    println!("Got 'sender'");
                }
                25 if name.eq_ignore_ascii_case(b"resent-message-id") => {
                    println!("Got 'resent-message-id'");
                }
                26 if name.eq_ignore_ascii_case(b"comments") => {
                    println!("Got 'comments'");
                }
                27 if name.eq_ignore_ascii_case(b"list-help") => {
                    println!("Got 'list-help'");
                }
                28 if name.eq_ignore_ascii_case(b"references") => {
                    println!("Got 'references'");
                }
                29 if name.eq_ignore_ascii_case(b"return-path") => {
                    println!("Got 'return-path'");
                }
                30 if name.eq_ignore_ascii_case(b"mime-version") => {
                    println!("Got 'mime-version'");
                }
                31 if name.eq_ignore_ascii_case(b"keywords") => {
                    println!("Got 'keywords'");
                }
                // content disposition!
                32 if name.eq_ignore_ascii_case(b"content-description") => {
                    println!("Got 'content-description'");
                }
                33 if name.eq_ignore_ascii_case(b"content-transfer-encoding") => {
                    println!("Got 'content-transfer-encoding'");
                }
                35 if name.eq_ignore_ascii_case(b"subject") => {
                    println!("Got 'subject'");
                }
                36 if name.eq_ignore_ascii_case(b"reply-to") => {
                    println!("Got 'reply-to'");
                }
                37 if name.eq_ignore_ascii_case(b"resent-to") => {
                    println!("Got 'resent-to'");
                }
                39 if name.eq_ignore_ascii_case(b"in-reply-to") => {
                    println!("Got 'in-reply-to'");
                }
                40 if name.eq_ignore_ascii_case(b"to") => {
                    println!("Got 'to'");
                }
                42 if name.eq_ignore_ascii_case(b"from") => {
                    println!("Got 'from'");
                }
                _ => return None,
            }
        }

        None
    }

    pub fn parse_hdr_name(stream: &'x MessageStream) -> Option<&'x [u8]> {
        let mut token_start: usize = 0;
        let mut token_end: usize = 0;

        while let Some(ch) = stream.next() {
            match ch {
                b':' => {
                    if token_start != 0 {
                        return stream.get_bytes(token_start - 1, token_end);
                    }
                }
                b' ' | b'\n' | b'\r' => (),
                _ => {
                    if token_start == 0 {
                        token_start = stream.get_pos();
                    }

                    token_end = stream.get_pos();
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::{header::Header, message_stream::MessageStream};

    #[test]
    fn parse_field_name() {
        let inputs = [
            ("From: ".to_string(), "from"),
            ("\n\n \nreceived-FROM: ".to_string(), "received-from"),
            (" subject   : ".to_string(), "subject"),
            ("X-Custom-Field : ".to_string(), "x-custom-field"),
            (" T : ".to_string(), "t"),
            ("mal formed: ".to_string(), "mal formed"),
        ];

        for input in inputs {
            match Header::parse_hdr_name(&MessageStream::new(input.0.as_bytes())) {
                Some(string) => {
                    assert_eq!(input.1, std::str::from_utf8(string).unwrap().to_lowercase())
                }
                None => panic!("Failed to parse '{}'", input.0),
            }
        }
    }
}

static HDR_HASH: &[u8] = &[
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 0, 0, 10, 0, 35, 10, 20, 0, 45, 5, 5, 5, 15, 30, 15, 45, 0, 20, 10, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
];
