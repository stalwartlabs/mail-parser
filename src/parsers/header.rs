use std::borrow::Cow;

use crate::parsers::fields::address::{Address, AddressList};
use crate::parsers::message_stream::MessageStream;

use super::fields::unstructured::parse_unstructured;

pub enum Header<'x> {
    Sender(Address<'x>),
    From(Address<'x>),
    To(AddressList<'x>),
    Cc(AddressList<'x>),
    Bcc(AddressList<'x>),
    Subject(Cow<'x, str>),
    Other((&'x str, &'x str)),
}

impl<'x> Header<'x> {

    pub fn parse(stream: &'x mut MessageStream) -> Option<Header<'x>> {
        while let Some(header) = Header::parse_hdr_name(stream) {
            match header {
                (0, b"cc") => {}
                (1, b"bcc") => {}
                (7, b"resent-cc") => {}
                (8, b"resent-bcc") => {}
                (9, b"resent-date") => {}
                (10, b"content-type") => {}
                (11, b"resent-sender") => {}
                (12, b"date") => {}
                (13, b"list-owner") => {}
                (14, b"resent-from") => {}
                (15, b"list-archive") => {}
                (16, b"received") => {}
                (17, b"list-subscribe") => {}
                (18, b"content-id") => {}
                (19, b"list-unsubscribe") => {}
                (22, b"list-post") => {}
                (23, b"message-id") => {}
                (24, b"sender") => {}
                (25, b"resent-message-id") => {}
                (26, b"comments") => {}
                (27, b"list-help") => {}
                (28, b"references") => {}
                (29, b"return-path") => {}
                (30, b"mime-version") => {}
                (31, b"keywords") => {}
                (32, b"content-description") => {}
                (33, b"content-transfer-encoding") => {}
                (35, b"subject") => return Some(Header::Subject(parse_unstructured(stream)?)),
                (36, b"reply-to") => {}
                (37, b"resent-to") => {}
                (39, b"in-reply-to") => {}
                (40, b"to") => {}
                (42, b"from") => {}
                _ => ()
            }
        }

        None
    }

    pub fn parse_hdr_name(stream: &'x mut MessageStream) -> Option<(u32, &'x [u8])> {
        let mut started = false;
        let mut first_ch: u8 = 0;
        let mut last_ch: u8 = 0;

        while let Some(ch) = stream.next_mut() {
            match ch {
                b':' => {
                    if started {
                        return Some((
                            stream.get_written_bytes() as u32
                                + unsafe {
                                    *HDR_HASH.get_unchecked(first_ch as usize) as u32
                                        + *HDR_HASH.get_unchecked(last_ch as usize) as u32
                                },
                            stream.end_write().unwrap(),
                        ));
                    }
                }
                b' ' | b'\n' | b'\r' => (),
                _ => {
                    if ch.is_ascii_uppercase() {
                        *ch += 32;
                    }

                    if !started {
                        first_ch = *ch;
                        stream.begin_write();
                        started = true;
                    } else {
                        last_ch = *ch;
                    }

                    stream.write_skip();
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
        unsafe {
            let inputs = [
                ("From: ".to_string(), "from"),
                ("\n\n \nreceived-FROM: ".to_string(), "received-from"),
                (" subject   : ".to_string(), "subject"),
                ("X-Custom-Field : ".to_string(), "x-custom-field"),
                ("mal formed: ".to_string(), "mal forme"), // Not expected to parse this
            ];

            for input in inputs {
                match Header::parse_hdr_name(&mut MessageStream::new(
                    input.0.clone().as_bytes_mut(),
                )) {
                    Some((_, string)) => assert_eq!(input.1, std::str::from_utf8(string).unwrap()),
                    None => panic!("Failed to parse '{}'", input.0),
                }
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
