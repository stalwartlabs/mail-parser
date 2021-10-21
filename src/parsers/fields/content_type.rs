use std::{borrow::Cow, collections::HashMap};
use serde::{Serialize, Deserialize};

use crate::{
    decoders::{
        buffer_writer::BufferWriter,
        charsets::map::{get_charset_decoder, get_default_decoder},
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

struct ContentTypeParser<'x> {
    state: ContentState,
    state_stack: Vec<ContentState>,

    c_type: Option<Cow<'x, str>>,
    c_subtype: Option<Cow<'x, str>>,
    attr_name: Option<Cow<'x, str>>,
    values: Vec<Cow<'x, str>>,
    attributes: HashMap<Cow<'x, str>, Cow<'x, str>>,

    token_start: usize,
    token_end: usize,

    is_encoded_attribute: bool,
    is_escaped: bool,
    is_lower_case: bool,
    is_token_safe: bool,
    is_token_start: bool,
}

fn add_attribute<'x>(parser: &mut ContentTypeParser<'x>, stream: &'x MessageStream) {
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
            ContentState::Type => parser.c_type = attr,
            ContentState::SubType => parser.c_subtype = attr,
            ContentState::AttributeName => parser.attr_name = attr,
            _ => unreachable!(),
        }

        parser.token_start = 0;
        parser.is_token_safe = true;
        parser.is_token_start = true;
    }
}

fn add_attribute_parameter<'x>(parser: &mut ContentTypeParser<'x>, stream: &'x MessageStream) {
    if parser.token_start > 0 {
        let attr_part = stream
            .get_string(
                parser.token_start - 1,
                parser.token_end,
                parser.is_token_safe,
            )
            .unwrap();
        let mut attr_name = parser.attr_name.as_ref().unwrap().clone() + "-charset";

        if parser.attributes.contains_key(&attr_name) {
            attr_name = parser.attr_name.as_ref().unwrap().clone() + "-language";
        }
        parser.attributes.insert(attr_name, attr_part);
        parser.token_start = 0;
        parser.is_token_safe = true;
    }
}

fn add_partial_value<'x>(
    parser: &mut ContentTypeParser<'x>,
    stream: &'x MessageStream,
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
        parser.token_start = 0;
        parser.is_token_safe = true;
    }
}

fn add_value<'x>(
    parser: &mut ContentTypeParser<'x>,
    stream: &'x MessageStream,
    buffer: &'x BufferWriter,
) {
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

    if !parser.is_encoded_attribute {
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

        if let Some(charset) = parser.attributes.get(&(attr_name.clone() + "-charset")) {
            if decode_hex(
                value.as_bytes(),
                (get_charset_decoder(charset.as_bytes(), buffer)
                    .unwrap_or_else(|| get_default_decoder(buffer)))
                .as_ref(),
            ) {
                if let Some(result) = buffer.get_string() {
                    value = result.into();
                }
            } else {
                buffer.reset_tail();
            }
        }

        let value = if let Some(old_value) = parser.attributes.get(&attr_name) {
            old_value.to_owned() + value
        } else {
            value
        };

        parser.attributes.insert(attr_name, value);
        parser.is_encoded_attribute = false;
    }

    if has_values {
        parser.values.clear();
    }

    parser.token_start = 0;
    parser.is_token_start = true;
    parser.is_token_safe = true;
}

pub fn parse_content_type<'x>(
    stream: &'x MessageStream,
    buffer: &'x BufferWriter,
) -> Option<ContentType<'x>> {
    let mut parser = ContentTypeParser {
        state: ContentState::Type,
        state_stack: Vec::new(),

        c_type: None,
        c_subtype: None,
        attr_name: None,
        attributes: HashMap::new(),
        values: Vec::new(),

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
                        add_attribute(&mut parser, stream)
                    }
                    ContentState::AttributeValue | ContentState::AttributeQuotedValue => {
                        add_value(&mut parser, stream, buffer)
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
                        }
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
                        add_value(&mut parser, stream, buffer);
                        parser.state = ContentState::AttributeName;
                    } else {
                        parser.is_escaped = false;
                    }
                    continue;
                }
                _ => (),
            },
            b'*' if parser.state == ContentState::AttributeName => {
                if !parser.is_encoded_attribute {
                    add_attribute(&mut parser, stream);
                    parser.is_encoded_attribute = true;
                }
                continue;
            }
            b'=' => match parser.state {
                ContentState::AttributeName => {
                    if !parser.is_encoded_attribute {
                        add_attribute(&mut parser, stream);
                    } else {
                        parser.token_start = 0;
                    }
                    parser.state = ContentState::AttributeValue;
                    continue;
                }
                ContentState::AttributeValue | ContentState::AttributeQuotedValue
                    if parser.is_token_start =>
                {
                    if let Some(token) = parse_encoded_word(stream, buffer) {
                        add_partial_value(&mut parser, stream, false);
                        parser.values.push(token.into());
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
                        add_value(&mut parser, stream, buffer);
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
                    && parser.state == ContentState::AttributeValue =>
            {
                add_attribute_parameter(&mut parser, stream);
                continue;
            }
            b'(' if parser.state != ContentState::AttributeQuotedValue => {
                if !parser.is_escaped {
                    match parser.state {
                        ContentState::Type
                        | ContentState::AttributeName
                        | ContentState::SubType => add_attribute(&mut parser, stream),
                        ContentState::AttributeValue => add_value(&mut parser, stream, buffer),
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
    use std::{borrow::Cow, collections::HashMap};

    use crate::{
        decoders::buffer_writer::BufferWriter,
        parsers::{fields::content_type::ContentType, message_stream::MessageStream},
    };

    use super::parse_content_type;

    #[test]
    fn parse_content_fields() {
        let inputs = [
            ("audio/basic\n", "audio||basic"),
            ("application/postscript \n", "application||postscript"),
            ("image/ jpeg\n", "image||jpeg"),
            (" message / rfc822\n", "message||rfc822"),
            ("inline\n", "inline"),
            (
                " text/plain; charset =us-ascii (Plain text)\n",
                "text||plain||charset~~us-ascii",
            ),
            (
                "text/plain; charset= \"us-ascii\"\n",
                "text||plain||charset~~us-ascii",
            ),
            (
                "text/plain; charset =ISO-8859-1\n",
                "text||plain||charset~~ISO-8859-1",
            ),
            ("text/foo; charset= bar\n", "text||foo||charset~~bar"),
            (
                " text /plain; charset=\"iso-8859-1\"; format=flowed\n",
                "text||plain||charset~~iso-8859-1||format~~flowed",
            ),
            (
                "application/pgp-signature; x-mac-type=70674453;\n    name=PGP.sig\n",
                "application||pgp-signature||x-mac-type~~70674453||name~~PGP.sig",
            ),
            (
                "multipart/mixed; boundary=gc0p4Jq0M2Yt08j34c0p\n",
                "multipart||mixed||boundary~~gc0p4Jq0M2Yt08j34c0p",
            ),
            (
                "multipart/mixed; boundary=gc0pJq0M:08jU534c0p\n",
                "multipart||mixed||boundary~~gc0pJq0M:08jU534c0p",
            ),
            (
                "multipart/mixed; boundary=\"gc0pJq0M:08jU534c0p\"\n",
                "multipart||mixed||boundary~~gc0pJq0M:08jU534c0p",
            ),
            (
                "multipart/mixed; boundary=\"simple boundary\"\n",
                "multipart||mixed||boundary~~simple boundary",
            ),
            (
                "multipart/alternative; boundary=boundary42\n",
                "multipart||alternative||boundary~~boundary42",
            ),
            (
                " multipart/mixed;\n     boundary=\"---- main boundary ----\"\n",
                "multipart||mixed||boundary~~---- main boundary ----",
            ),
            (
                "multipart/alternative; boundary=42\n",
                "multipart||alternative||boundary~~42",
            ),
            (
                "message/partial; id=\"ABC@host.com\";\n",
                "message||partial||id~~ABC@host.com",
            ),
            (
                "multipart/parallel;boundary=unique-boundary-2\n",
                "multipart||parallel||boundary~~unique-boundary-2",
            ),
            (
                concat!(
                    "message/external-body; name=\"BodyFormats.ps\";\n   site=\"thumper.bellcor",
                    "e.com\"; mode=\"image\";\n  access-type=ANON-FTP; directory=\"pub\";\n  expir",
                    "ation=\"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n"
                ),
                concat!(
                    "message||external-body||name~~BodyFormats.ps||site~~thumper.bellcore.c",
                    "om||mode~~image||access-type~~ANON-FTP||directory~~pub||expiration~~Fr",
                    "i, 14 Jun 1991 19:13:14 -0400 (EDT)"
                ),
            ),
            (
                concat!(
                    "message/external-body; access-type=local-file;\n   name=\"/u/nsb/writing",
                    "/rfcs/RFC-MIME.ps\";\n    site=\"thumper.bellcore.com\";\n  expiration=\"Fri",
                    ", 14 Jun 1991 19:13:14 -0400 (EDT)\"\n"
                ),
                concat!(
                    "message||external-body||access-type~~local-file||expiration~~Fri, 14 J",
                    "un 1991 19:13:14 -0400 (EDT)||name~~/u/nsb/writing/rfcs/RFC-MIME.ps||s",
                    "ite~~thumper.bellcore.com"
                ),
            ),
            (
                concat!(
                    "message/external-body;\n    access-type=mail-server\n     server=\"listse",
                    "rv@bogus.bitnet\";\n     expiration=\"Fri, 14 Jun 1991 19:13:14 -0400 (ED",
                    "T)\"\n"
                ),
                concat!(
                    "message||external-body||access-type~~mail-server||server~~listserv@bog",
                    "us.bitnet||expiration~~Fri, 14 Jun 1991 19:13:14 -0400 (EDT)"
                ),
            ),
            (
                concat!(
                    "Message/Partial; number=2; total=3;\n     id=\"oc=jpbe0M2Yt4s@thumper.be",
                    "llcore.com\"\n"
                ),
                concat!(
                    "message||partial||number~~2||total~~3||id~~oc=jpbe0M2Yt4s@thumper.bell",
                    "core.com"
                ),
            ),
            (
                concat!(
                    "multipart/signed; micalg=pgp-sha1; protocol=\"application/pgp-signature",
                    "\";\n   boundary=\"=-J1qXPoyGtE2XNN5N6Z6j\"\n"
                ),
                concat!(
                    "multipart||signed||protocol~~application/pgp-signature||boundary~~=-J1",
                    "qXPoyGtE2XNN5N6Z6j||micalg~~pgp-sha1"
                ),
            ),
            (
                concat!(
                    "message/external-body;\n    access-type=local-file;\n     name=\"/u/nsb/M",
                    "e.jpeg\"\n"
                ),
                concat!("message||external-body||access-type~~local-file||name~~/u/nsb/Me.jpeg"),
            ),
            (
                concat!(
                    "message/external-body; access-type=URL;\n    URL*0=\"ftp://\";\n    URL*1=",
                    "\"cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar\"\n"
                ),
                concat!(
                    "message||external-body||url~~ftp://cs.utk.edu/pub/moore/bulk-mailer/bu",
                    "lk-mailer.tar||access-type~~URL"
                ),
            ),
            (
                concat!(
                    "message/external-body; access-type=URL;\n     URL=\"ftp://cs.utk.edu/pub",
                    "/moore/bulk-mailer/bulk-mailer.tar\"\n"
                ),
                concat!(
                    "message||external-body||access-type~~URL||url~~ftp://cs.utk.edu/pub/mo",
                    "ore/bulk-mailer/bulk-mailer.tar"
                ),
            ),
            (
                concat!(
                    "application/x-stuff;\n     title*=us-ascii'en-us'This%20is%20%2A%2A%2Af",
                    "un%2A%2A%2A\n"
                ),
                concat!(
                    "application||x-stuff||title-language~~en-us||title~~This is ***fun***|",
                    "|title-charset~~us-ascii"
                ),
            ),
            (
                concat!(
                    "application/x-stuff\n   title*0*=us-ascii'en'This%20is%20even%20more%20",
                    "\n   title*1*=%2A%2A%2Afun%2A%2A%2A%20\n   title*2=\"isn't it!\"\n"
                ),
                concat!(
                    "application||x-stuff||title~~This is even more ***fun*** isn't it!||ti",
                    "tle-charset~~us-ascii||title-language~~en"
                ),
            ),
            (
                concat!(
                    "application/pdf\n   filename*0*=iso-8859-1'es'%D1and%FA\n   filename*1*=",
                    "%20r%E1pido\n   filename*2=\" (versi%F3n \\'99 \\\"oficial\\\").pdf\"\n"
                ),
                concat!(
                    "application||pdf||filename~~Ñandú rápido (versión '99 \"oficial\").pdf||",
                    "filename-charset~~iso-8859-1||filename-language~~es"
                ),
            ),
            (
                concat!(
                    " image/png;\n   name=\"=?utf-8?q?=E3=83=8F=E3=83=AD=E3=83=BC=E3=83=BB=E3",
                    "=83=AF=E3=83=BC=E3=83=AB=E3=83=89?=.png\"\n"
                ),
                concat!("image||png||name~~ハロー・ワールド.png"),
            ),
            (
                " image/gif;\n   name==?iso-8859-6?b?5dHNyMcgyMfk2cfk5Q==?=.gif\n",
                "image||gif||name~~مرحبا بالعالم.gif",
            ),
            (
                concat!(
                    "image/jpeg;\n   name=\"=?iso-8859-1?B?4Q==?= =?utf-8?B?w6k=?= =?iso-8859",
                    "-1?q?=ED?=.jpeg\"\n"
                ),
                concat!("image||jpeg||name~~á é í.jpeg"),
            ),
            (
                concat!(
                    "image/jpeg;\n   name==?iso-8859-1?B?4Q==?= =?utf-8?B?w6k=?= =?iso-8859-",
                    "1?q?=ED?=.jpeg\n"
                ),
                concat!("image||jpeg||name~~áéí.jpeg"),
            ),
            (
                "image/gif;\n   name==?iso-8859-6?b?5dHNyMcgyMfk2cfk5S5naWY=?=\n",
                "image||gif||name~~مرحبا بالعالم.gif",
            ),
            (
                " image/gif;\n   name=\"=?iso-8859-6?b?5dHNyMcgyMfk2cfk5S5naWY=?=\"\n",
                "image||gif||name~~مرحبا بالعالم.gif",
            ),
            (
                " inline; filename=\"  best \\\"file\\\" ever with \\\\ escaped ' stuff.  \"\n",
                "inline||||filename~~  best \"file\" ever with \\ escaped ' stuff.  ",
            ),
            ("test/\n", "test"),
            ("/invalid\n", ""),
            ("/\n", ""),
            (";\n", ""),
            ("/ ; name=value\n", ""),
            ("text/plain;\n", "text||plain"),
            ("text/plain;;\n", "text||plain"),
            (
                "text/plain ;;;;; = ;; name=\"value\"\n",
                "text||plain||name~~value",
            ),
            ("=\n", "="),
            ("name=value\n", "name=value"),
            ("text/plain; name=  \n", "text||plain"),
            ("a/b; = \n", "a||b"),
            ("a/b; = \n \n", "a||b"),
            ("a/b; =value\n", "a||b"),
            ("test/test; =\"value\"\n", "test||test"),
            ("á/é; á=é\n", "á||é||á~~é"),
            ("inva/lid; name=\"   \n", "inva||lid||name~~   "),
            ("inva/lid; name=\"   \n    \n", "inva||lid||name~~   "),
            (
                "inva/lid; name=\"   \n    \"; test=test\n",
                "inva||lid||name~~   ||test~~test",
            ),
            ("name=value\n", "name=value"),
        ];

        for input in inputs {
            let stream = MessageStream::new(input.0.as_bytes());
            let buffer = BufferWriter::with_capacity(input.0.len() * 2);
            let result = parse_content_type(&stream, &buffer);
            let expected = if !input.1.is_empty() {
                let mut c_type: Option<Cow<str>> = None;
                let mut c_subtype: Option<Cow<str>> = None;
                let mut attributes: HashMap<Cow<str>, Cow<str>> = HashMap::new();

                for (count, part) in input.1.split("||").enumerate() {
                    match count {
                        0 => c_type = Some(part.into()),
                        1 => {
                            c_subtype = if part.is_empty() {
                                None
                            } else {
                                Some(part.into())
                            }
                        }
                        _ => {
                            let attr: Vec<&str> = part.split("~~").collect();
                            attributes.insert(attr[0].into(), attr[1].into());
                        }
                    }
                }

                c_type.map(|content_type| ContentType {
                    c_type: content_type,
                    c_subtype: c_subtype.take(),
                    attributes: if !attributes.is_empty() {
                        Some(attributes)
                    } else {
                        None
                    },
                })
            } else {
                None
            };

            assert_eq!(result, expected, "Failed for '{:?}'", input.0);
        }
    }
}
