use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    ops::Add,
};

use crate::{
    decoders::{
        charsets::map::{decoder_default, get_charset_decoder},
        encoded_word::parse_encoded_word,
        hex::decode_hex,
    },
    parsers::message_stream::MessageStream,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentType<'x> {
    c_type: Cow<'x, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    c_subtype: Option<Cow<'x, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    attributes: Option<HashMap<Cow<'x, str>, Cow<'x, str>>>,
}

impl<'x> ContentType<'x> {
    pub fn get_type(&'x self) -> &'x str {
        &self.c_type
    }

    pub fn get_subtype(&'x self) -> Option<&'x str> {
        self.c_subtype.as_ref()?.as_ref().into()
    }

    pub fn get_attribute(&'x self, name: &str) -> Option<&'x str> {
        self.attributes.as_ref()?.get(name)?.as_ref().into()
    }

    pub fn has_attribute(&'x self, name: &str) -> bool {
        self.attributes
            .as_ref()
            .map_or_else(|| false, |attr| attr.contains_key(name))
    }

    pub fn is_attachment(&'x self) -> bool {
        self.c_type.eq_ignore_ascii_case("attachment")
    }

    pub fn is_inline(&'x self) -> bool {
        self.c_type.eq_ignore_ascii_case("inline")
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ContentState {
    Type,
    SubType,
    AttributeName,
    AttributeValue,
    AttributeQuotedValue,
    Comment,
}

type Continuation<'x> = (Cow<'x, str>, u32, Cow<'x, str>);

struct ContentTypeParser<'x> {
    state: ContentState,
    state_stack: Vec<ContentState>,

    c_type: Option<Cow<'x, str>>,
    c_subtype: Option<Cow<'x, str>>,

    attr_name: Option<Cow<'x, str>>,
    attr_charset: Option<Cow<'x, str>>,
    attr_position: u32,

    values: Vec<Cow<'x, str>>,
    attributes: HashMap<Cow<'x, str>, Cow<'x, str>>,
    continuations: Option<Vec<Continuation<'x>>>,

    token_start: usize,
    token_end: usize,

    is_continuation: bool,
    is_encoded_attribute: bool,
    is_escaped: bool,
    is_lower_case: bool,
    is_token_safe: bool,
    is_token_start: bool,
}

#[inline(always)]
fn reset_parser(parser: &mut ContentTypeParser) {
    parser.token_start = 0;
    parser.is_token_safe = true;
    parser.is_token_start = true;
}

fn add_attribute<'x>(parser: &mut ContentTypeParser<'x>, stream: &MessageStream<'x>) -> bool {
    if parser.token_start > 0 {
        let mut attr = stream.get_string(
            parser.token_start - 1,
            parser.token_end,
            parser.is_token_safe,
        );

        if !parser.is_lower_case {
            attr.as_mut().unwrap().to_mut().make_ascii_lowercase();
            parser.is_lower_case = true;
        }

        match parser.state {
            ContentState::AttributeName => parser.attr_name = attr,
            ContentState::Type => parser.c_type = attr,
            ContentState::SubType => parser.c_subtype = attr,
            _ => unreachable!(),
        }

        reset_parser(parser);
        true
    } else {
        false
    }
}

fn add_attribute_parameter<'x>(parser: &mut ContentTypeParser<'x>, stream: &MessageStream<'x>) {
    if parser.token_start > 0 {
        let attr_part = stream
            .get_string(
                parser.token_start - 1,
                parser.token_end,
                parser.is_token_safe,
            )
            .unwrap();

        if parser.attr_charset.is_none() {
            parser.attr_charset = attr_part.into();
        } else if let Entry::Vacant(e) = parser.attributes.entry(
            parser
                .attr_name
                .as_ref()
                .unwrap_or(&"unknown".into())
                .clone()
                + "-language",
        ) {
            e.insert(attr_part);
        } else {
            parser.values.push("'".into());
            parser.values.push(attr_part);
        }

        reset_parser(parser);
    }
}

fn add_partial_value<'x>(
    parser: &mut ContentTypeParser<'x>,
    stream: &MessageStream<'x>,
    to_cur_pos: bool,
) {
    if parser.token_start > 0 {
        let in_quote = parser.state == ContentState::AttributeQuotedValue;

        parser.values.push(
            stream
                .get_string(
                    parser.token_start - 1,
                    if in_quote && to_cur_pos {
                        stream.get_pos() - 1
                    } else {
                        parser.token_end
                    },
                    parser.is_token_safe,
                )
                .unwrap(),
        );
        if !in_quote {
            parser.values.push(" ".into());
        }

        reset_parser(parser);
    }
}

fn add_value<'x>(parser: &mut ContentTypeParser<'x>, stream: &MessageStream<'x>) {
    if parser.attr_name.is_none() {
        return;
    }

    let has_values = !parser.values.is_empty();
    let value = if parser.token_start > 0 {
        stream.get_string(
            parser.token_start - 1,
            parser.token_end,
            parser.is_token_safe,
        )
    } else {
        if !has_values {
            return;
        }
        None
    };

    if !parser.is_continuation {
        parser.attributes.insert(
            parser.attr_name.take().unwrap(),
            if !has_values {
                value.unwrap()
            } else {
                if let Some(value) = value {
                    parser.values.push(value);
                }
                parser.values.concat().into()
            },
        );
    } else {
        let attr_name = parser.attr_name.take().unwrap();
        let mut value = if let Some(value) = value {
            if has_values {
                Cow::from(parser.values.concat()) + value
            } else {
                value
            }
        } else {
            parser.values.concat().into()
        };

        if parser.is_encoded_attribute {
            if let (true, decoded_bytes) = decode_hex(value.as_bytes()) {
                value = get_charset_decoder(
                    parser
                        .attr_charset
                        .as_ref()
                        .unwrap_or(&"utf-8".into())
                        .as_bytes(),
                )
                .unwrap_or(decoder_default)(&decoded_bytes)
                .into_owned()
                .into();
            }
            parser.is_encoded_attribute = false;
        }

        if parser.attr_position > 0 {
            let continuation = (attr_name, parser.attr_position, value);
            if let Some(continuations) = parser.continuations.as_mut() {
                continuations.push(continuation);
            } else {
                parser.continuations = Some(vec![continuation]);
            }

            parser.attr_position = 0;
        } else {
            parser.attributes.insert(attr_name, value);
        }
        parser.is_continuation = false;
        parser.attr_charset = None;
    }

    if has_values {
        parser.values.clear();
    }

    reset_parser(parser);
}

fn add_attr_position(parser: &mut ContentTypeParser, stream: &MessageStream) -> bool {
    if parser.token_start > 0 {
        parser.attr_position = stream
            .get_string(
                parser.token_start - 1,
                parser.token_end,
                parser.is_token_safe,
            )
            .unwrap()
            .parse()
            .unwrap_or(0);

        reset_parser(parser);
        true
    } else {
        false
    }
}

fn merge_continuations(parser: &mut ContentTypeParser) {
    let continuations = parser.continuations.as_mut().unwrap();
    continuations.sort();
    for (key, _, value) in continuations.drain(..) {
        let value = if let Some(old_value) = parser.attributes.get(&key) {
            old_value.to_owned() + value
        } else {
            value
        };
        parser.attributes.insert(key, value);
    }
}

pub fn parse_content_type<'x>(stream: &MessageStream<'x>) -> Option<ContentType<'x>> {
    let mut parser = ContentTypeParser {
        state: ContentState::Type,
        state_stack: Vec::new(),

        c_type: None,
        c_subtype: None,

        attr_name: None,
        attr_charset: None,
        attr_position: 0,

        attributes: HashMap::new(),
        values: Vec::new(),
        continuations: None,

        is_continuation: false,
        is_encoded_attribute: false,
        is_lower_case: true,
        is_token_safe: true,
        is_token_start: true,
        is_escaped: false,

        token_start: 0,
        token_end: 0,
    };

    while let Some(ch) = stream.next() {
        match ch {
            b' ' | b'\t' => {
                if !parser.is_token_start {
                    parser.is_token_start = true;
                }
                if let ContentState::AttributeQuotedValue = parser.state {
                    if parser.token_start == 0 {
                        parser.token_start = stream.get_pos();
                        parser.token_end = parser.token_start;
                    } else {
                        parser.token_end = stream.get_pos();
                    }
                }
                continue;
            }
            b'A'..=b'Z' => {
                if parser.is_lower_case {
                    if let ContentState::Type
                    | ContentState::SubType
                    | ContentState::AttributeName = parser.state
                    {
                        parser.is_lower_case = false;
                    }
                }
            }
            b'\n' => {
                match parser.state {
                    ContentState::Type | ContentState::AttributeName | ContentState::SubType => {
                        add_attribute(&mut parser, stream);
                    }
                    ContentState::AttributeValue | ContentState::AttributeQuotedValue => {
                        add_value(&mut parser, stream);
                    }
                    _ => (),
                }

                match stream.peek() {
                    Some(b' ' | b'\t') => {
                        parser.state = ContentState::AttributeName;
                        stream.advance(1);

                        if !parser.is_token_start {
                            parser.is_token_start = true;
                        }
                        continue;
                    }
                    _ => {
                        if parser.continuations.is_some() {
                            merge_continuations(&mut parser);
                        }
                        return if let Some(content_type) = parser.c_type {
                            Some(ContentType {
                                c_type: content_type,
                                c_subtype: parser.c_subtype.take(),
                                attributes: if !parser.attributes.is_empty() {
                                    Some(parser.attributes)
                                } else {
                                    None
                                },
                            })
                        } else {
                            None
                        };
                    }
                }
            }
            b'/' if parser.state == ContentState::Type => {
                add_attribute(&mut parser, stream);
                parser.state = ContentState::SubType;
                continue;
            }
            b';' => match parser.state {
                ContentState::Type | ContentState::SubType | ContentState::AttributeName => {
                    add_attribute(&mut parser, stream);
                    parser.state = ContentState::AttributeName;
                    continue;
                }
                ContentState::AttributeValue => {
                    if !parser.is_escaped {
                        add_value(&mut parser, stream);
                        parser.state = ContentState::AttributeName;
                    } else {
                        parser.is_escaped = false;
                    }
                    continue;
                }
                _ => (),
            },
            b'*' if parser.state == ContentState::AttributeName => {
                if !parser.is_continuation {
                    parser.is_continuation = add_attribute(&mut parser, stream);
                } else if !parser.is_encoded_attribute {
                    add_attr_position(&mut parser, stream);
                    parser.is_encoded_attribute = true;
                } else {
                    // Malformed data, reset parser.
                    reset_parser(&mut parser);
                }
                continue;
            }
            b'=' => match parser.state {
                ContentState::AttributeName => {
                    if !parser.is_continuation {
                        if !add_attribute(&mut parser, stream) {
                            continue;
                        }
                    } else if !parser.is_encoded_attribute {
                        /* If is_continuation=true && is_encoded_attribute=false,
                        the last character was a '*' which means encoding */
                        parser.is_encoded_attribute = !add_attr_position(&mut parser, stream);
                    } else {
                        reset_parser(&mut parser);
                    }
                    parser.state = ContentState::AttributeValue;
                    continue;
                }
                ContentState::AttributeValue | ContentState::AttributeQuotedValue
                    if parser.is_token_start =>
                {
                    if let Some(token) = parse_encoded_word(stream) {
                        add_partial_value(&mut parser, stream, false);
                        parser.values.push(token);
                        continue;
                    }
                }
                _ => (),
            },
            b'\"' => match parser.state {
                ContentState::AttributeValue => {
                    if !parser.is_token_start {
                        parser.is_token_start = true;
                    }
                    parser.state = ContentState::AttributeQuotedValue;
                    continue;
                }
                ContentState::AttributeQuotedValue => {
                    if !parser.is_escaped {
                        add_value(&mut parser, stream);
                        parser.state = ContentState::AttributeName;
                        continue;
                    } else {
                        parser.is_escaped = false;
                    }
                }
                _ => continue,
            },
            b'\\' => match parser.state {
                ContentState::AttributeQuotedValue | ContentState::AttributeValue => {
                    if !parser.is_escaped {
                        add_partial_value(&mut parser, stream, true);
                        parser.is_escaped = true;
                        continue;
                    } else {
                        parser.is_escaped = false;
                    }
                }
                ContentState::Comment => parser.is_escaped = !parser.is_escaped,
                _ => continue,
            },
            b'\''
                if parser.is_encoded_attribute
                    && !parser.is_escaped
                    && (parser.state == ContentState::AttributeValue
                        || parser.state == ContentState::AttributeQuotedValue) =>
            {
                add_attribute_parameter(&mut parser, stream);
                continue;
            }
            b'(' if parser.state != ContentState::AttributeQuotedValue => {
                if !parser.is_escaped {
                    match parser.state {
                        ContentState::Type
                        | ContentState::AttributeName
                        | ContentState::SubType => {
                            add_attribute(&mut parser, stream);
                        }
                        ContentState::AttributeValue => {
                            add_value(&mut parser, stream);
                        }
                        _ => (),
                    }

                    parser.state_stack.push(parser.state);
                    parser.state = ContentState::Comment;
                } else {
                    parser.is_escaped = false;
                }
                continue;
            }
            b')' if parser.state == ContentState::Comment => {
                if !parser.is_escaped {
                    parser.state = parser.state_stack.pop().unwrap();
                    reset_parser(&mut parser);
                } else {
                    parser.is_escaped = false;
                }
                continue;
            }
            b'\r' => continue,
            0..=0x7f => (),
            _ => {
                if parser.is_token_safe {
                    parser.is_token_safe = false;
                }
            }
        }

        if parser.is_escaped {
            parser.is_escaped = false;
        }

        if parser.is_token_start {
            parser.is_token_start = false;
        }

        if parser.token_start == 0 {
            parser.token_start = stream.get_pos();
            parser.token_end = parser.token_start;
        } else {
            parser.token_end = stream.get_pos();
        }
    }

    None
}

mod tests {
    #[test]
    fn parse_content_fields() {
        use crate::parsers::{fields::content_type::ContentType, message_stream::MessageStream};

        use super::parse_content_type;

        let inputs = [
            (
                "audio/basic\n",
                concat!("---\n", "c_type: audio\n", "c_subtype: basic\n"),
            ),
            (
                "application/postscript \n",
                concat!("---\n", "c_type: application\n", "c_subtype: postscript\n"),
            ),
            (
                "image/ jpeg\n",
                concat!("---\n", "c_type: image\n", "c_subtype: jpeg\n"),
            ),
            (
                " message / rfc822\n",
                concat!("---\n", "c_type: message\n", "c_subtype: rfc822\n"),
            ),
            ("inline\n", concat!("---\n", "c_type: inline\n")),
            (
                " text/plain; charset =us-ascii (Plain text)\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "text/plain; charset= \"us-ascii\"\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "text/plain; charset =ISO-8859-1\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: ISO-8859-1\n"
                ),
            ),
            (
                "text/foo; charset= bar\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: foo\n",
                    "attributes:\n",
                    "  charset: bar\n"
                ),
            ),
            (
                " text /plain; charset=\"iso-8859-1\"; format=flowed\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: iso-8859-1\n",
                    "  format: flowed\n"
                ),
            ),
            (
                "application/pgp-signature; x-mac-type=70674453;\n    name=PGP.sig\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: pgp-signature\n",
                    "attributes:\n",
                    "  name: PGP.sig\n",
                    "  x-mac-type: \"70674453\"\n"
                ),
            ),
            (
                "multipart/mixed; boundary=gc0p4Jq0M2Yt08j34c0p\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: mixed\n",
                    "attributes:\n",
                    "  boundary: gc0p4Jq0M2Yt08j34c0p\n"
                ),
            ),
            (
                "multipart/mixed; boundary=gc0pJq0M:08jU534c0p\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: mixed\n",
                    "attributes:\n",
                    "  boundary: \"gc0pJq0M:08jU534c0p\"\n"
                ),
            ),
            (
                "multipart/mixed; boundary=\"gc0pJq0M:08jU534c0p\"\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: mixed\n",
                    "attributes:\n",
                    "  boundary: \"gc0pJq0M:08jU534c0p\"\n"
                ),
            ),
            (
                "multipart/mixed; boundary=\"simple boundary\"\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: mixed\n",
                    "attributes:\n",
                    "  boundary: simple boundary\n"
                ),
            ),
            (
                "multipart/alternative; boundary=boundary42\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: alternative\n",
                    "attributes:\n",
                    "  boundary: boundary42\n"
                ),
            ),
            (
                " multipart/mixed;\n     boundary=\"---- main boundary ----\"\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: mixed\n",
                    "attributes:\n",
                    "  boundary: \"---- main boundary ----\"\n"
                ),
            ),
            (
                "multipart/alternative; boundary=42\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: alternative\n",
                    "attributes:\n",
                    "  boundary: \"42\"\n"
                ),
            ),
            (
                "message/partial; id=\"ABC@host.com\";\n",
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: partial\n",
                    "attributes:\n",
                    "  id: ABC@host.com\n"
                ),
            ),
            (
                "multipart/parallel;boundary=unique-boundary-2\n",
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: parallel\n",
                    "attributes:\n",
                    "  boundary: unique-boundary-2\n"
                ),
            ),
            (
                concat!(
                    "message/external-body; name=\"BodyFormats.ps\";\n   site=\"thumper.bellcor",
                    "e.com\"; mode=\"image\";\n  access-type=ANON-FTP; directory=\"pub\";\n  expir",
                    "ation=\"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: external-body\n",
                    "attributes:\n",
                    "  expiration: \"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n",
                    "  site: thumper.bellcore.com\n",
                    "  directory: pub\n",
                    "  name: BodyFormats.ps\n",
                    "  mode: image\n",
                    "  access-type: ANON-FTP\n"
                ),
            ),
            (
                concat!(
                    "message/external-body; access-type=local-file;\n   name=\"/u/nsb/writing",
                    "/rfcs/RFC-MIME.ps\";\n    site=\"thumper.bellcore.com\";\n  expiration=\"Fri",
                    ", 14 Jun 1991 19:13:14 -0400 (EDT)\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: external-body\n",
                    "attributes:\n",
                    "  expiration: \"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n",
                    "  access-type: local-file\n",
                    "  name: /u/nsb/writing/rfcs/RFC-MIME.ps\n",
                    "  site: thumper.bellcore.com\n"
                ),
            ),
            (
                concat!(
                    "message/external-body;\n    access-type=mail-server\n     server=\"listse",
                    "rv@bogus.bitnet\";\n     expiration=\"Fri, 14 Jun 1991 19:13:14 -0400 (ED",
                    "T)\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: external-body\n",
                    "attributes:\n",
                    "  access-type: mail-server\n",
                    "  server: listserv@bogus.bitnet\n",
                    "  expiration: \"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n"
                ),
            ),
            (
                concat!(
                    "Message/Partial; number=2; total=3;\n     id=\"oc=jpbe0M2Yt4s@thumper.be",
                    "llcore.com\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: partial\n",
                    "attributes:\n",
                    "  total: \"3\"\n",
                    "  number: \"2\"\n",
                    "  id: oc=jpbe0M2Yt4s@thumper.bellcore.com\n"
                ),
            ),
            (
                concat!(
                    "multipart/signed; micalg=pgp-sha1; protocol=\"application/pgp-signature",
                    "\";\n   boundary=\"=-J1qXPoyGtE2XNN5N6Z6j\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: multipart\n",
                    "c_subtype: signed\n",
                    "attributes:\n",
                    "  boundary: \"=-J1qXPoyGtE2XNN5N6Z6j\"\n",
                    "  micalg: pgp-sha1\n",
                    "  protocol: application/pgp-signature\n"
                ),
            ),
            (
                concat!(
                    "message/external-body;\n    access-type=local-file;\n     name=\"/u/nsb/M",
                    "e.jpeg\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: external-body\n",
                    "attributes:\n",
                    "  name: /u/nsb/Me.jpeg\n",
                    "  access-type: local-file\n"
                ),
            ),
            (
                concat!(
                    "message/external-body; access-type=URL;\n    URL*0=\"ftp://\";\n    URL*1=",
                    "\"cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: external-body\n",
                    "attributes:\n",
                    "  url: \"ftp://cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar\"\n",
                    "  access-type: URL\n"
                ),
            ),
            (
                concat!(
                    "message/external-body; access-type=URL;\n     URL=\"ftp://cs.utk.edu/pub",
                    "/moore/bulk-mailer/bulk-mailer.tar\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: external-body\n",
                    "attributes:\n",
                    "  url: \"ftp://cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar\"\n",
                    "  access-type: URL\n"
                ),
            ),
            (
                concat!(
                    "application/x-stuff;\n     title*=us-ascii'en-us'This%20is%20%2A%2A%2Af",
                    "un%2A%2A%2A\n"
                ),
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: x-stuff\n",
                    "attributes:\n",
                    "  title: This is ***fun***\n",
                    "  title-language: en-us\n"
                ),
            ),
            (
                concat!(
                    "application/x-stuff\n   title*0*=us-ascii'en'This%20is%20even%20more%20",
                    "\n   title*1*=%2A%2A%2Afun%2A%2A%2A%20\n   title*2=\"isn't it!\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: x-stuff\n",
                    "attributes:\n",
                    "  title-language: en\n",
                    "  title: \"This is even more ***fun*** isn't it!\"\n"
                ),
            ),
            (
                concat!(
                    "application/pdf\n   filename*0*=iso-8859-1'es'%D1and%FA\n   ",
                    "filename*1*=iso-8859-1'",
                    "%20r%E1pido\n   filename*2*=\"iso-8859-1' ",
                    "(versi%F3n \\'99 \\\"oficial\\\").pdf\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: pdf\n",
                    "attributes:\n",
                    "  filename: \"Ñandú rápido (versión '99 \\\"oficial\\\").pdf\"\n",
                    "  filename-language: es\n",
                ),
            ),
            (
                concat!(
                    " image/png;\n   name=\"=?utf-8?q?=E3=83=8F=E3=83=AD=E3=83=BC=E3=83=BB=E3",
                    "=83=AF=E3=83=BC=E3=83=AB=E3=83=89?=.png\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: image\n",
                    "c_subtype: png\n",
                    "attributes:\n",
                    "  name: ハロー・ワールド.png\n"
                ),
            ),
            (
                " image/gif;\n   name==?iso-8859-6?b?5dHNyMcgyMfk2cfk5Q==?=.gif\n",
                concat!(
                    "---\n",
                    "c_type: image\n",
                    "c_subtype: gif\n",
                    "attributes:\n",
                    "  name: مرحبا بالعالم.gif\n"
                ),
            ),
            (
                concat!(
                    "image/jpeg;\n   name=\"=?iso-8859-1?B?4Q==?= =?utf-8?B?w6k=?= =?iso-8859",
                    "-1?q?=ED?=.jpeg\"\n"
                ),
                concat!(
                    "---\n",
                    "c_type: image\n",
                    "c_subtype: jpeg\n",
                    "attributes:\n",
                    "  name: á é í.jpeg\n"
                ),
            ),
            (
                concat!(
                    "image/jpeg;\n   name==?iso-8859-1?B?4Q==?= =?utf-8?B?w6k=?= =?iso-8859-",
                    "1?q?=ED?=.jpeg\n"
                ),
                concat!(
                    "---\n",
                    "c_type: image\n",
                    "c_subtype: jpeg\n",
                    "attributes:\n",
                    "  name: áéí.jpeg\n"
                ),
            ),
            (
                "image/gif;\n   name==?iso-8859-6?b?5dHNyMcgyMfk2cfk5S5naWY=?=\n",
                concat!(
                    "---\n",
                    "c_type: image\n",
                    "c_subtype: gif\n",
                    "attributes:\n",
                    "  name: مرحبا بالعالم.gif\n"
                ),
            ),
            (
                " image/gif;\n   name=\"=?iso-8859-6?b?5dHNyMcgyMfk2cfk5S5naWY=?=\"\n",
                concat!(
                    "---\n",
                    "c_type: image\n",
                    "c_subtype: gif\n",
                    "attributes:\n",
                    "  name: مرحبا بالعالم.gif\n"
                ),
            ),
            (
                " inline; filename=\"  best \\\"file\\\" ever with \\\\ escaped ' stuff.  \"\n",
                concat!(
                    "---\n",
                    "c_type: inline\n",
                    "attributes:\n",
                    "  filename: \"  best \\\"file\\\" ever with \\\\ escaped ' stuff.  \"\n"
                ),
            ),
            ("test/\n", concat!("---\n", "c_type: test\n")),
            ("/invalid\n", concat!("---\n", "~\n")),
            ("/\n", concat!("---\n", "~\n")),
            (";\n", concat!("---\n", "~\n")),
            ("/ ; name=value\n", concat!("---\n", "~\n")),
            (
                "text/plain;\n",
                concat!("---\n", "c_type: text\n", "c_subtype: plain\n"),
            ),
            (
                "text/plain;;\n",
                concat!("---\n", "c_type: text\n", "c_subtype: plain\n"),
            ),
            (
                "text/plain ;;;;; = ;; name=\"value\"\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  name: value\n"
                ),
            ),
            ("=\n", concat!("---\n", "c_type: \"=\"\n")),
            ("name=value\n", concat!("---\n", "c_type: name=value\n")),
            (
                "text/plain; name=  \n",
                concat!("---\n", "c_type: text\n", "c_subtype: plain\n"),
            ),
            (
                "a/b; = \n",
                concat!("---\n", "c_type: a\n", "c_subtype: b\n"),
            ),
            (
                "a/b; = \n \n",
                concat!("---\n", "c_type: a\n", "c_subtype: b\n"),
            ),
            (
                "a/b; =value\n",
                concat!("---\n", "c_type: a\n", "c_subtype: b\n"),
            ),
            (
                "test/test; =\"value\"\n",
                concat!("---\n", "c_type: test\n", "c_subtype: test\n"),
            ),
            (
                "á/é; á=é\n",
                concat!(
                    "---\n",
                    "c_type: á\n",
                    "c_subtype: é\n",
                    "attributes:\n",
                    "  á: é\n"
                ),
            ),
            (
                "inva/lid; name=\"   \n",
                concat!(
                    "---\n",
                    "c_type: inva\n",
                    "c_subtype: lid\n",
                    "attributes:\n",
                    "  name: \"   \"\n"
                ),
            ),
            (
                "inva/lid; name=\"   \n    \n",
                concat!(
                    "---\n",
                    "c_type: inva\n",
                    "c_subtype: lid\n",
                    "attributes:\n",
                    "  name: \"   \"\n"
                ),
            ),
            (
                "inva/lid; name=\"   \n    \"; test=test\n",
                concat!(
                    "---\n",
                    "c_type: inva\n",
                    "c_subtype: lid\n",
                    "attributes:\n",
                    "  test: test\n",
                    "  name: \"   \"\n"
                ),
            ),
            ("name=value\n", concat!("---\n", "c_type: name=value\n")),
            (
                concat!(
                    "test/encoded; key4*=us-ascii''foo; key*9999=ba%; key2*0=a; key3*0*=us-asc",
                    "ii'en'xyz; key*0=\"f\u{0}oo\"; key2*1*=b%25; key3*1=plop%; key*1=baz; ",
                    "*=test; =test2;\n"
                ),
                concat!(
                    "---\n",
                    "c_type: test\n",
                    "c_subtype: encoded\n",
                    "attributes:\n",
                    "  key: \"f\\u0000oobazba%\"\n",
                    "  key2: ab%\n",
                    "  key3: xyzplop%\n",
                    "  key4: foo\n",
                    "  key3-language: en\n",
                ),
            ),
            (
                "text/plain; name*=\"iso-8859-1''HasenundFr%F6sche.txt\"\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  name: HasenundFrösche.txt\n"
                ),
            ),
            (
                concat!(
                    "malicious/attempt; 1*2*=a; 3**=b; 4***=c; *5**6*=d;",
                    "*****7*8*=e; 9*10*11*12*13*14=f; 15*x***fff*===g;",
                    "1 * 999999999999999999 *= h; 18 *=*=*=*=*==i;\n"
                ),
                concat!(
                    "---\n",
                    "c_type: malicious\n",
                    "c_subtype: attempt\n",
                    "attributes:\n",
                    "  \"1\": ha\n",
                    "  \"3\": b\n",
                    "  \"4\": c\n",
                    "  \"5\": d\n",
                    "  \"7\": e\n",
                    "  \"9\": f\n",
                    "  \"15\": \"==g\"\n",
                    "  \"18\": \"*=*=*=*==i\"\n",
                ),
            ),
            (";charset=us-ascii\n", concat!("---\n", "~\n")),
            (" ;charset=us-ascii\n", concat!("---\n", "~\n")),
            ("/\n", concat!("---\n", "~\n")),
            ("/;charset=us-ascii\n", concat!("---\n", "~\n")),
            ("/ ;charset=us-ascii\n", concat!("---\n", "~\n")),
            ("text/\n", concat!("---\n", "c_type: text\n")),
            (
                "text/;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "text/ ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            ("/plain\n", concat!("---\n", "~\n")),
            ("/plain;charset=us-ascii\n", concat!("---\n", "~\n")),
            ("/plain ;charset=us-ascii\n", concat!("---\n", "~\n")),
            (
                "text/plain\n",
                concat!("---\n", "c_type: text\n", "c_subtype: plain\n"),
            ),
            (
                "text/plain;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "text/plain ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "text/plain/format\n",
                concat!("---\n", "c_type: text\n", "c_subtype: plain/format\n"),
            ),
            (
                "text/plain/format;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain/format\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "text/plain/format ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain/format\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/ld+json\n",
                concat!("---\n", "c_type: application\n", "c_subtype: ld+json\n"),
            ),
            (
                "application/ld+json;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: ld+json\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/ld+json ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: ld+json\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/x-magic-cap-package-1.0\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: x-magic-cap-package-1.0\n"
                ),
            ),
            (
                "application/x-magic-cap-package-1.0;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: x-magic-cap-package-1.0\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/x-magic-cap-package-1.0 ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: x-magic-cap-package-1.0\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/pro_eng\n",
                concat!("---\n", "c_type: application\n", "c_subtype: pro_eng\n"),
            ),
            (
                "application/pro_eng;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: pro_eng\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/pro_eng ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: pro_eng\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/wordperfect6.1\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: wordperfect6.1\n"
                ),
            ),
            (
                "application/wordperfect6.1;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: wordperfect6.1\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "application/wordperfect6.1 ;charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: wordperfect6.1\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                concat!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.templat",
                    "e\n"
                ),
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: vnd.openxmlformats-officedocument.wordprocessingml.template\n"
                ),
            ),
            (
                concat!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.templat",
                    "e;charset=us-ascii\n"
                ),
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: vnd.openxmlformats-officedocument.wordprocessingml.template\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                concat!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.templat",
                    "e ;charset=us-asii\n"
                ),
                concat!(
                    "---\n",
                    "c_type: application\n",
                    "c_subtype: vnd.openxmlformats-officedocument.wordprocessingml.template\n",
                    "attributes:\n",
                    "  charset: us-asii\n"
                ),
            ),
            (
                "(hello) text (plain) / (world) plain (eod)\n",
                concat!("---\n", "c_type: text\n", "c_subtype: plain\n"),
            ),
            (
                "(hello) text (plain) / (world) plain (eod);charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "(hello) text (plain) / (world) plain (eod); charset=us-ascii\n",
                concat!(
                    "---\n",
                    "c_type: text\n",
                    "c_subtype: plain\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                "message/rfc822\r\n\n",
                concat!("---\n", "c_type: message\n", "c_subtype: rfc822\n"),
            ),
            (
                " \t\r message/rfc822 \t\r\n\n",
                concat!("---\n", "c_type: message\n", "c_subtype: rfc822\n"),
            ),
            (
                " \t\r message/rfc822 \t ;charset=us-ascii\r\n\n",
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: rfc822\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
            (
                " \t\r message/rfc822 \t ; charset=us-ascii\r\n",
                concat!(
                    "---\n",
                    "c_type: message\n",
                    "c_subtype: rfc822\n",
                    "attributes:\n",
                    "  charset: us-ascii\n"
                ),
            ),
        ];

        for input in inputs {
            let mut str = input.0.to_string();

            let result = parse_content_type(&MessageStream::new(unsafe { str.as_bytes_mut() }));
            let expected: Option<ContentType> = serde_yaml::from_str(input.1).unwrap_or(None);

            /*
            if input.0.len() >= 70 {
                println!(
                    "(concat!({:?}), concat!({:?})),",
                    input
                        .0
                        .chars()
                        .collect::<Vec<char>>()
                        .chunks(70)
                        .map(|c| c.iter().collect::<String>())
                        .collect::<Vec<String>>(),
                    serde_yaml::to_string(&result)
                        .unwrap_or("".to_string())
                        .split_inclusive("\n")
                        .collect::<Vec<&str>>()
                );
            } else {
                println!(
                    "({:?}, concat!({:?})),",
                    input.0,
                    serde_yaml::to_string(&result)
                        .unwrap_or("".to_string())
                        .split_inclusive("\n")
                        .collect::<Vec<&str>>()
                );
            }*/

            assert_eq!(
                result,
                expected,
                "Failed for '{:?}', result was:\n{}",
                input.0,
                serde_yaml::to_string(result.as_ref().unwrap()).unwrap()
            );
        }
    }
}
