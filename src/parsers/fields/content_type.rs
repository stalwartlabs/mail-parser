use std::{borrow::Cow, collections::HashMap};

use crate::{
    decoders::{
        charsets::{parser::CharsetParser, utf8::Utf8Decoder},
        hex::HexDecoder,
        Decoder, DecoderResult,
    },
    parsers::{encoded_word::parse_encoded_word, message_stream::MessageStream},
};

#[derive(Clone, Copy, PartialEq, Debug)]
enum ContentState {
    Type,
    SubType,
    AttributeName,
    AttributeValue,
    AttributeQuotedValue,
    Comment,
}

#[derive(PartialEq, Debug)]
pub struct ContentType<'x> {
    c_type: Option<Cow<'x, str>>,
    c_subtype: Option<Cow<'x, str>>,
    attributes: HashMap<Cow<'x, str>, Cow<'x, str>>,
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

impl<'x> ContentTypeParser<'x> {
    fn new() -> ContentTypeParser<'x> {
        ContentTypeParser {
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
        }
    }
}

fn add_attribute<'x>(
    mut parser: ContentTypeParser<'x>,
    stream: &'x MessageStream,
) -> ContentTypeParser<'x> {
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
    parser
}

fn add_value<'x>(
    mut parser: ContentTypeParser<'x>,
    stream: &'x MessageStream,
) -> ContentTypeParser<'x> {
    if parser.attr_name.is_none() {
        return parser;
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
            return parser;
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
                Cow::from(parser.values.concat())
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
            Cow::from(parser.values.concat())
        };

        if let Some(charset) = parser.attributes.get(&(attr_name.clone() + "-charset")) {
            let mut decoder = CharsetParser::new();
            decoder.ingest_slice(charset.as_bytes());
            let mut charset_decoder = decoder
                .get_decoder(75)
                .unwrap_or_else(|| Box::new(Utf8Decoder::new(75)));
            let mut qp_decoder = HexDecoder::new();

            for ch in value.as_bytes() {
                match qp_decoder.ingest(ch) {
                    DecoderResult::Byte(b) => charset_decoder.ingest(&b),
                    DecoderResult::ByteArray(ba) => charset_decoder.ingest_slice(ba),
                    DecoderResult::NeedData => (),
                    DecoderResult::Error => break,
                }
            }

            if let Some(result) = charset_decoder.to_string() {
                value = result.into();
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
    parser
}

pub fn parse_content_type<'x>(stream: &'x MessageStream) -> Option<ContentType<'x>> {
    let mut parser = ContentTypeParser::new();

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
                    ContentState::Type | ContentState::AttributeName | ContentState::SubType
                        if parser.token_start > 0 =>
                    {
                        parser = add_attribute(parser, stream)
                    }
                    ContentState::AttributeValue | ContentState::AttributeQuotedValue => {
                        parser = add_value(parser, stream)
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
                    _ if parser.c_type.is_some() => {
                        return Some(ContentType {
                            c_type: parser.c_type,
                            c_subtype: parser.c_subtype,
                            attributes: parser.attributes,
                        });
                    }
                    _ => return None,
                }
            }
            b'/' if parser.state == ContentState::Type => {
                if parser.token_start > 0 {
                    parser = add_attribute(parser, stream);
                }
                parser.state = ContentState::SubType;
                continue;
            }
            b';' => match parser.state {
                ContentState::Type | ContentState::SubType | ContentState::AttributeName => {
                    if parser.token_start > 0 {
                        parser = add_attribute(parser, stream);
                    }
                    parser.state = ContentState::AttributeName;
                    continue;
                }
                ContentState::AttributeValue => {
                    if !parser.is_escaped {
                        parser = add_value(parser, stream);
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
                    if parser.token_start > 0 {
                        parser = add_attribute(parser, stream);
                    }
                    parser.is_encoded_attribute = true;
                }
                continue;
            }
            b'=' => match parser.state {
                ContentState::AttributeName => {
                    if parser.token_start > 0 {
                        if !parser.is_encoded_attribute {
                            parser = add_attribute(parser, stream);
                        } else {
                            parser.token_start = 0;
                        }
                    }
                    parser.state = ContentState::AttributeValue;

                    continue;
                }
                ContentState::AttributeValue | ContentState::AttributeQuotedValue
                    if parser.is_token_start && stream.skip_byte(&b'?') =>
                {
                    let pos_back = stream.get_pos() - 1;

                    if let Some(token) = parse_encoded_word(stream) {
                        if parser.token_start > 0 {
                            parser.values.push(
                                stream
                                    .get_string(
                                        parser.token_start - 1,
                                        parser.token_end,
                                        parser.is_token_safe,
                                    )
                                    .unwrap(),
                            );
                            if parser.state != ContentState::AttributeQuotedValue {
                                parser.values.push(Cow::from(" "));
                            }
                            parser.token_start = 0;
                            parser.is_token_safe = true;
                        }
                        parser.values.push(Cow::from(token));
                        continue;
                    } else {
                        stream.set_pos(pos_back);
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
                        parser = add_value(parser, stream);
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
                        if parser.token_start > 0 {
                            parser.values.push(
                                stream
                                    .get_string(
                                        parser.token_start - 1,
                                        if parser.state == ContentState::AttributeQuotedValue {
                                            stream.get_pos() - 1
                                        } else {
                                            parser.token_end
                                        },
                                        parser.is_token_safe,
                                    )
                                    .unwrap(),
                            );
                            parser.token_start = 0;
                            parser.is_token_safe = true;
                        }
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
                continue;
            }
            b'(' if parser.state != ContentState::AttributeQuotedValue => {
                if !parser.is_escaped {
                    match parser.state {
                        ContentState::Type
                        | ContentState::AttributeName
                        | ContentState::SubType
                            if parser.token_start > 0 =>
                        {
                            parser = add_attribute(parser, stream)
                        }
                        ContentState::AttributeValue => parser = add_value(parser, stream),
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
    use std::collections::HashMap;

    use crate::parsers::message_stream::MessageStream;

    use super::{parse_content_type, ContentType};

    #[test]
    fn parse_content_fields() {
        let inputs = [
            (
                "audio/basic\n".to_string(), 
                "audio||basic".to_string()),
            (
                "application/postscript \n".to_string(), 
                "application||postscript".to_string()),
            (
                "image/ jpeg\n".to_string(), 
                "image||jpeg".to_string()),
            (
                " message / rfc822\n".to_string(), 
                "message||rfc822".to_string()),
            (
                "inline\n".to_string(), 
                "inline".to_string()),
            (
                " text/plain; charset =us-ascii (Plain text)\n".to_string(), 
                "text||plain||charset~~us-ascii".to_string()),
            (
                "text/plain; charset= \"us-ascii\"\n".to_string(), 
                "text||plain||charset~~us-ascii".to_string()),
            (
                "text/plain; charset =ISO-8859-1\n".to_string(), 
                "text||plain||charset~~ISO-8859-1".to_string()),
            (
                "text/foo; charset= bar\n".to_string(), 
                "text||foo||charset~~bar".to_string()),
            (
                " text /plain; charset=\"iso-8859-1\"; format=flowed\n".to_string(), 
                "text||plain||charset~~iso-8859-1||format~~flowed".to_string()),
            (
                "application/pgp-signature; x-mac-type=70674453;\n    name=PGP.sig\n".to_string(), 
                "application||pgp-signature||x-mac-type~~70674453||name~~PGP.sig".to_string()),
            (
                "multipart/mixed; boundary=gc0p4Jq0M2Yt08j34c0p\n".to_string(), 
                "multipart||mixed||boundary~~gc0p4Jq0M2Yt08j34c0p".to_string()),
            (
                "multipart/mixed; boundary=gc0pJq0M:08jU534c0p\n".to_string(), 
                "multipart||mixed||boundary~~gc0pJq0M:08jU534c0p".to_string()),
            (
                "multipart/mixed; boundary=\"gc0pJq0M:08jU534c0p\"\n".to_string(), 
                "multipart||mixed||boundary~~gc0pJq0M:08jU534c0p".to_string()),
            (
                "multipart/mixed; boundary=\"simple boundary\"\n".to_string(), 
                "multipart||mixed||boundary~~simple boundary".to_string()),
            (
                "multipart/alternative; boundary=boundary42\n".to_string(), 
                "multipart||alternative||boundary~~boundary42".to_string()),
            (
                " multipart/mixed;\n     boundary=\"---- main boundary ----\"\n".to_string(), 
                "multipart||mixed||boundary~~---- main boundary ----".to_string()),
            (
                "multipart/alternative; boundary=42\n".to_string(), 
                "multipart||alternative||boundary~~42".to_string()),
            (
                "message/partial; id=\"ABC@host.com\";\n".to_string(), 
                "message||partial||id~~ABC@host.com".to_string()),
            (
                "multipart/parallel;boundary=unique-boundary-2\n".to_string(), 
                "multipart||parallel||boundary~~unique-boundary-2".to_string()),
            (
                "message/external-body; name=\"BodyFormats.ps\";\n   site=\"thumper.bellcore.com\"; mode=\"image\";\n  access-type=ANON-FTP; directory=\"pub\";\n  expiration=\"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n".to_string(), 
                "message||external-body||name~~BodyFormats.ps||site~~thumper.bellcore.com||mode~~image||access-type~~ANON-FTP||directory~~pub||expiration~~Fri, 14 Jun 1991 19:13:14 -0400 (EDT)".to_string()),
            (
                "message/external-body; access-type=local-file;\n   name=\"/u/nsb/writing/rfcs/RFC-MIME.ps\";\n    site=\"thumper.bellcore.com\";\n  expiration=\"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n".to_string(), 
                "message||external-body||access-type~~local-file||expiration~~Fri, 14 Jun 1991 19:13:14 -0400 (EDT)||name~~/u/nsb/writing/rfcs/RFC-MIME.ps||site~~thumper.bellcore.com".to_string()),
            (
                "message/external-body;\n    access-type=mail-server\n     server=\"listserv@bogus.bitnet\";\n     expiration=\"Fri, 14 Jun 1991 19:13:14 -0400 (EDT)\"\n".to_string(), 
                "message||external-body||access-type~~mail-server||server~~listserv@bogus.bitnet||expiration~~Fri, 14 Jun 1991 19:13:14 -0400 (EDT)".to_string()),
            (
                "Message/Partial; number=2; total=3;\n     id=\"oc=jpbe0M2Yt4s@thumper.bellcore.com\"\n".to_string(), 
                "message||partial||number~~2||total~~3||id~~oc=jpbe0M2Yt4s@thumper.bellcore.com".to_string()),
            (
                "multipart/signed; micalg=pgp-sha1; protocol=\"application/pgp-signature\";\n   boundary=\"=-J1qXPoyGtE2XNN5N6Z6j\"\n".to_string(), 
                "multipart||signed||protocol~~application/pgp-signature||boundary~~=-J1qXPoyGtE2XNN5N6Z6j||micalg~~pgp-sha1".to_string()),
            (
                "message/external-body;\n    access-type=local-file;\n     name=\"/u/nsb/Me.jpeg\"\n".to_string(), 
                "message||external-body||access-type~~local-file||name~~/u/nsb/Me.jpeg".to_string()
            ),
            (
                "message/external-body; access-type=URL;\n    URL*0=\"ftp://\";\n    URL*1=\"cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar\"\n".to_string(),
                "message||external-body||url~~ftp://cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar||access-type~~URL".to_string()
            ),
            (
                "message/external-body; access-type=URL;\n     URL=\"ftp://cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar\"\n".to_string(), 
                "message||external-body||access-type~~URL||url~~ftp://cs.utk.edu/pub/moore/bulk-mailer/bulk-mailer.tar".to_string()),
            (
                "application/x-stuff;\n     title*=us-ascii\'en-us\'This%20is%20%2A%2A%2Afun%2A%2A%2A\n".to_string(), 
                "application||x-stuff||title-language~~en-us||title~~This is ***fun***||title-charset~~us-ascii".to_string()
            ),
            (
                "application/x-stuff\n   title*0*=us-ascii\'en\'This%20is%20even%20more%20\n   title*1*=%2A%2A%2Afun%2A%2A%2A%20\n   title*2=\"isn't it!\"\n".to_string(), 
                "application||x-stuff||title~~This is even more ***fun*** isn't it!||title-charset~~us-ascii||title-language~~en".to_string()
            ),
            (
                "application/pdf\n   filename*0*=iso-8859-1\'es\'%D1and%FA\n   filename*1*=%20r%E1pido\n   filename*2=\" (versi%F3n \\\'99 \\\"oficial\\\").pdf\"\n".to_string(), 
                "application||pdf||filename~~Ñandú rápido (versión \'99 \"oficial\").pdf||filename-charset~~iso-8859-1||filename-language~~es".to_string()
            ),
            (
                " image/png;\n   name=\"=?utf-8?q?=E3=83=8F=E3=83=AD=E3=83=BC=E3=83=BB=E3=83=AF=E3=83=BC=E3=83=AB=E3=83=89?=.png\"\n".to_string(), 
                "image||png||name~~ハロー・ワールド.png".to_string()),
            (
                " image/gif;\n   name==?iso-8859-6?b?5dHNyMcgyMfk2cfk5Q==?=.gif\n".to_string(), 
                "image||gif||name~~مرحبا بالعالم.gif".to_string()),
            (
                "image/jpeg;\n   name=\"=?iso-8859-1?B?4Q==?= =?utf-8?B?w6k=?= =?iso-8859-1?q?=ED?=.jpeg\"\n".to_string(), 
                "image||jpeg||name~~á é í.jpeg".to_string()),
            (
                "image/jpeg;\n   name==?iso-8859-1?B?4Q==?= =?utf-8?B?w6k=?= =?iso-8859-1?q?=ED?=.jpeg\n".to_string(), 
                "image||jpeg||name~~áéí.jpeg".to_string()),
            (
                "image/gif;\n   name==?iso-8859-6?b?5dHNyMcgyMfk2cfk5S5naWY=?=\n".to_string(), 
                "image||gif||name~~مرحبا بالعالم.gif".to_string()),
            (
                " image/gif;\n   name=\"=?iso-8859-6?b?5dHNyMcgyMfk2cfk5S5naWY=?=\"\n".to_string(), 
                "image||gif||name~~مرحبا بالعالم.gif".to_string()),
            (
                " inline; filename=\"  best \\\"file\\\" ever with \\\\ escaped \\' stuff.  \"\n".to_string(), 
                "inline||||filename~~  best \"file\" ever with \\ escaped ' stuff.  ".to_string()
            ),
            (
                "test/\n".to_string(), 
                "test".to_string()
            ),
            (
                "/invalid\n".to_string(), 
                "".to_string()),
            (
                "/\n".to_string(), 
                "".to_string()),
            (
                ";\n".to_string(), 
                "".to_string()),
            (
                "/ ; name=value\n".to_string(),
                "".to_string()),
            (
                "text/plain;\n".to_string(), 
                "text||plain".to_string()),
            (
                "text/plain;;\n".to_string(), 
                "text||plain".to_string()),
            (
                "text/plain ;;;;; = ;; name=\"value\"\n".to_string(), 
                "text||plain||name~~value".to_string()),
            (
                "=\n".to_string(), 
                "=".to_string()),
            (
                "name=value\n".to_string(), 
                "name=value".to_string()),
            (
                "text/plain; name=  \n".to_string(), 
                "text||plain".to_string()),
            (
                "a/b; = \n".to_string(), 
                "a||b".to_string()),
            (
                "a/b; = \n \n".to_string(), 
                "a||b".to_string()),
            (
                "a/b; =value\n".to_string(), 
                "a||b".to_string()),
            (
                "test/test; =\"value\"\n".to_string(), 
                "test||test".to_string()),
            (
                "á/é; á=é\n".to_string(), 
                "á||é||á~~é".to_string()),
            (
                "inva/lid; name=\"   \n".to_string(), 
                "inva||lid||name~~   ".to_string()),
            (
                "inva/lid; name=\"   \n    \n".to_string(), 
                "inva||lid||name~~   ".to_string()),
            (
                "inva/lid; name=\"   \n    \"; test=test\n".to_string(), 
                "inva||lid||name~~   ||test~~test".to_string()),
            (
                "name=value\n".to_string(), 
                "name=value".to_string()),

        ];

        for input in inputs {
            if let Some(result) = parse_content_type(&MessageStream::new(input.0.as_bytes())) {
                let mut ct = ContentType {
                    c_type: None,
                    c_subtype: None,
                    attributes: HashMap::new(),
                };
                for (count, part) in input.1.split("||").enumerate() {
                    match count {
                        0 => ct.c_type = Some(part.into()),
                        1 => {
                            ct.c_subtype = if part.is_empty() {
                                None
                            } else {
                                Some(part.into())
                            }
                        }
                        _ => {
                            let attr: Vec<&str> = part.split("~~").collect();
                            ct.attributes.insert(attr[0].into(), attr[1].into());
                        }
                    }
                }
                assert_eq!(result, ct, "failed for {}", input.0.escape_debug());
            } else {
                assert!(input.1.is_empty(), "failed for {}", input.0.escape_debug())
            }
        }
    }
}
