/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::borrow::Cow;

use crate::{
    Attribute, ContentType, HeaderValue,
    decoders::{
        charsets::{DecoderFnc, map::charset_decoder},
        hex::decode_hex,
    },
    parsers::MessageStream,
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

/// A single RFC 2231 continuation section of a parameter value. The charset
/// decode is deferred until every section has been collected, so it holds the
/// section's raw payload rather than a decoded string.
struct Continuation<'x> {
    name: Cow<'x, str>,
    position: u32,
    part: ContinuationPart<'x>,
    /// Charset label parsed from this section's own `charset'lang'` prefix.
    charset: Option<Cow<'x, str>>,
}

enum ContinuationPart<'x> {
    /// Percent-decoded raw octets of an encoded (`name*N*=`) section.
    Encoded(Vec<u8>),
    /// Verbatim text of a literal (`name*N=`) section.
    Literal(Cow<'x, str>),
}

struct ContentTypeParser<'x> {
    state: ContentState,
    state_stack: Vec<ContentState>,

    c_type: Option<Cow<'x, str>>,
    c_subtype: Option<Cow<'x, str>>,

    attr_name: Option<Cow<'x, str>>,
    attr_charset: Option<Cow<'x, str>>,
    attr_position: u32,

    values: Vec<Cow<'x, str>>,
    attributes: Vec<Attribute<'x>>,
    continuations: Option<Vec<Continuation<'x>>>,

    token_start: usize,
    token_end: usize,

    is_continuation: bool,
    is_encoded_attribute: bool,
    is_escaped: bool,
    remove_crlf: bool,
    is_lower_case: bool,
    is_token_start: bool,
}

impl<'x> ContentTypeParser<'x> {
    #[inline(always)]
    fn reset_parser(&mut self) {
        self.token_start = 0;
        self.is_token_start = true;
    }

    fn add_attribute(&mut self, stream: &MessageStream<'x>) -> bool {
        if self.token_start > 0 {
            let mut attr = Some(String::from_utf8_lossy(
                &stream.data[self.token_start - 1..self.token_end],
            ));

            if !self.is_lower_case {
                attr.as_mut().unwrap().to_mut().make_ascii_lowercase();
                self.is_lower_case = true;
            }

            match self.state {
                ContentState::AttributeName => self.attr_name = attr,
                ContentState::Type => self.c_type = attr,
                ContentState::SubType => self.c_subtype = attr,
                _ => unreachable!(),
            }

            self.reset_parser();
            true
        } else {
            false
        }
    }

    fn add_attribute_parameter(&mut self, stream: &MessageStream<'x>) {
        if self.token_start > 0 {
            let attr_part =
                String::from_utf8_lossy(&stream.data[self.token_start - 1..self.token_end]);

            if self.attr_charset.is_none() {
                self.attr_charset = attr_part.into();
            } else {
                let attr_name =
                    self.attr_name.as_ref().unwrap_or(&"unknown".into()).clone() + "-language";

                if !self.attributes.iter().any(|a| a.name == attr_name) {
                    self.attributes.push(Attribute {
                        name: attr_name,
                        value: attr_part,
                    });
                } else {
                    self.values.push("'".into());
                    self.values.push(attr_part);
                }
            }

            self.reset_parser();
        }
    }

    fn add_partial_value(&mut self, stream: &MessageStream<'x>, to_cur_pos: bool) {
        if self.token_start > 0 {
            let in_quote = self.state == ContentState::AttributeQuotedValue;

            self.values.push(String::from_utf8_lossy(
                &stream.data[self.token_start - 1..if in_quote && to_cur_pos {
                    stream.offset() - 1
                } else {
                    self.token_end
                }],
            ));
            if !in_quote {
                self.values.push(" ".into());
            }

            self.reset_parser();
        }
    }

    fn add_value(&mut self, stream: &MessageStream<'x>) {
        if self.attr_name.is_none() {
            return;
        }

        let has_values = !self.values.is_empty();
        let value = if self.token_start > 0 {
            let value = &stream.data[self.token_start - 1..self.token_end];
            Some(if !self.remove_crlf {
                String::from_utf8_lossy(value)
            } else {
                self.remove_crlf = false;
                match String::from_utf8(
                    value
                        .iter()
                        .filter(|&&ch| ch != b'\r' && ch != b'\n')
                        .copied()
                        .collect::<Vec<_>>(),
                ) {
                    Ok(value) => value.into(),
                    Err(err) => String::from_utf8_lossy(err.as_bytes()).into_owned().into(),
                }
            })
        } else {
            if !has_values {
                return;
            }
            None
        };

        if !self.is_continuation {
            self.attributes.push(Attribute {
                name: self.attr_name.take().unwrap(),
                value: if !has_values {
                    value.unwrap()
                } else {
                    if let Some(value) = value {
                        self.values.push(value);
                    }
                    self.values.concat().into()
                },
            });
        } else {
            let attr_name = self.attr_name.take().unwrap();
            let value = if let Some(value) = value {
                if has_values {
                    Cow::from(self.values.concat()) + value
                } else {
                    value
                }
            } else {
                self.values.concat().into()
            };

            // Keep the raw payload and defer the charset decode to
            // `merge_continuations`: percent-decode each encoded section to
            // octets now, concatenate every section's octets, then decode once.
            // Decoding a section in isolation mangles a multi-octet character
            // split across sections (RFC 2231 §4.1) and loses the charset that
            // only section 0 carries.
            let part = if self.is_encoded_attribute {
                self.is_encoded_attribute = false;
                match decode_hex(value.as_bytes()) {
                    (true, decoded_bytes) => ContinuationPart::Encoded(decoded_bytes),
                    (false, _) => ContinuationPart::Literal(value),
                }
            } else {
                ContinuationPart::Literal(value)
            };

            let continuation = Continuation {
                name: attr_name.clone(),
                position: self.attr_position,
                part,
                charset: self.attr_charset.take(),
            };
            if let Some(continuations) = self.continuations.as_mut() {
                continuations.push(continuation);
            } else {
                self.continuations = Some(vec![continuation]);
            }

            // Anchor section 0 in `attributes` at its parse position so the
            // merged parameter keeps its original ordering; the decoded value
            // is filled in by `merge_continuations`.
            if self.attr_position == 0 {
                self.attributes.push(Attribute {
                    name: attr_name,
                    value: Cow::default(),
                });
            }

            self.attr_position = 0;
            self.is_continuation = false;
        }

        if has_values {
            self.values.clear();
        }

        self.reset_parser();
    }

    fn add_attr_position(&mut self, stream: &MessageStream<'_>) -> bool {
        if self.token_start > 0 {
            self.attr_position =
                String::from_utf8_lossy(&stream.data[self.token_start - 1..self.token_end])
                    .parse()
                    .unwrap_or(0);

            self.reset_parser();
            true
        } else {
            false
        }
    }

    fn merge_continuations(&mut self) {
        let mut continuations = self.continuations.take().unwrap();
        continuations.sort_by(|a, b| a.name.cmp(&b.name).then(a.position.cmp(&b.position)));

        let mut idx = 0;
        while idx < continuations.len() {
            let end = idx
                + continuations[idx..]
                    .iter()
                    .take_while(|c| c.name == continuations[idx].name)
                    .count();
            let group = &continuations[idx..end];

            // The `charset'lang'` prefix belongs on section 0 (RFC 2231 §4.1);
            // fall back to any section that carries one, since some mailers
            // repeat it on every section.
            let decoder = group
                .iter()
                .find(|c| c.position == 0 && c.charset.is_some())
                .or_else(|| group.iter().find(|c| c.charset.is_some()))
                .and_then(|c| c.charset.as_ref())
                .and_then(|c| charset_decoder(c.as_bytes()));

            let mut result = String::new();
            let mut octets: Vec<u8> = Vec::new();
            for c in group {
                match &c.part {
                    ContinuationPart::Encoded(bytes) => octets.extend_from_slice(bytes),
                    ContinuationPart::Literal(text) => {
                        Self::flush_octets(&mut result, &mut octets, decoder);
                        result.push_str(text);
                    }
                }
            }
            Self::flush_octets(&mut result, &mut octets, decoder);

            let name = continuations[idx].name.clone();
            if let Some(anchor) = self.attributes.iter_mut().find(|a| a.name == name) {
                anchor.value = result.into();
            } else {
                self.attributes.push(Attribute {
                    name,
                    value: result.into(),
                });
            }
            idx = end;
        }
    }

    /// Charset-decode the concatenated octets collected so far and append them
    /// to `result`. Without a resolved charset, fall back to UTF-8 like the
    /// per-section path did.
    fn flush_octets(result: &mut String, octets: &mut Vec<u8>, decoder: Option<DecoderFnc>) {
        if octets.is_empty() {
            return;
        }
        if let Some(decoder) = decoder {
            result.push_str(&decoder(octets));
        } else {
            match std::str::from_utf8(octets) {
                Ok(s) => result.push_str(s),
                Err(_) => result.push_str(&String::from_utf8_lossy(octets)),
            }
        }
        octets.clear();
    }
}

impl<'x> MessageStream<'x> {
    pub fn parse_content_type(&mut self) -> HeaderValue<'x> {
        let mut parser = ContentTypeParser {
            state: ContentState::Type,
            state_stack: Vec::new(),

            c_type: None,
            c_subtype: None,

            attr_name: None,
            attr_charset: None,
            attr_position: 0,

            attributes: Vec::new(),
            values: Vec::new(),
            continuations: None,

            is_continuation: false,
            is_encoded_attribute: false,
            is_lower_case: true,
            is_token_start: true,
            is_escaped: false,
            remove_crlf: false,

            token_start: 0,
            token_end: 0,
        };

        while let Some(ch) = self.next() {
            match ch {
                b' ' | b'\t' => {
                    if !parser.is_token_start {
                        parser.is_token_start = true;
                    }
                    if let ContentState::AttributeQuotedValue = parser.state {
                        if parser.token_start == 0 {
                            parser.token_start = self.offset();
                            parser.token_end = parser.token_start;
                        } else {
                            parser.token_end = self.offset();
                        }
                    }
                    continue;
                }
                b'A'..=b'Z' if parser.is_lower_case => {
                    if let ContentState::Type
                    | ContentState::SubType
                    | ContentState::AttributeName = parser.state
                    {
                        parser.is_lower_case = false;
                    }
                }
                b'\n' => {
                    let next_is_space = self.peek_next_is_space();
                    match parser.state {
                        ContentState::Type
                        | ContentState::AttributeName
                        | ContentState::SubType => {
                            parser.add_attribute(self);
                        }
                        ContentState::AttributeValue => {
                            parser.add_value(self);
                        }
                        ContentState::AttributeQuotedValue => {
                            if next_is_space {
                                self.next();
                                parser.remove_crlf = true;
                                continue;
                            } else {
                                parser.add_value(self);
                            }
                        }
                        _ => (),
                    }

                    if next_is_space {
                        if parser.state == ContentState::Type {
                            continue;
                        }
                        parser.state = ContentState::AttributeName;
                        self.next();

                        if !parser.is_token_start {
                            parser.is_token_start = true;
                        }
                        continue;
                    } else {
                        if parser.continuations.is_some() {
                            parser.merge_continuations();
                        }

                        return if let Some(content_type) = parser.c_type {
                            HeaderValue::ContentType(ContentType {
                                c_type: content_type,
                                c_subtype: parser.c_subtype.take(),
                                attributes: if !parser.attributes.is_empty() {
                                    Some(parser.attributes)
                                } else {
                                    None
                                },
                            })
                        } else {
                            HeaderValue::Empty
                        };
                    }
                }
                b'/' if parser.state == ContentState::Type => {
                    parser.add_attribute(self);
                    parser.state = ContentState::SubType;
                    continue;
                }
                b';' => match parser.state {
                    ContentState::Type | ContentState::SubType | ContentState::AttributeName => {
                        parser.add_attribute(self);
                        parser.state = ContentState::AttributeName;
                        continue;
                    }
                    ContentState::AttributeValue => {
                        if !parser.is_escaped {
                            parser.add_value(self);
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
                        parser.is_continuation = parser.add_attribute(self);
                    } else if !parser.is_encoded_attribute {
                        parser.add_attr_position(self);
                        parser.is_encoded_attribute = true;
                    } else {
                        // Malformed data, reset parser.
                        parser.reset_parser();
                    }
                    continue;
                }
                b'=' => match parser.state {
                    ContentState::AttributeName => {
                        if !parser.is_continuation {
                            if !parser.add_attribute(self) {
                                continue;
                            }
                        } else if !parser.is_encoded_attribute {
                            /* If is_continuation=true && is_encoded_attribute=false,
                            the last character was a '*' which means encoding */
                            parser.is_encoded_attribute = !parser.add_attr_position(self);
                        } else {
                            parser.reset_parser();
                        }
                        parser.state = ContentState::AttributeValue;
                        continue;
                    }
                    ContentState::AttributeValue | ContentState::AttributeQuotedValue
                        if parser.is_token_start && self.peek_char(b'?') =>
                    {
                        self.checkpoint();
                        if let Some(token) = self.decode_rfc2047() {
                            parser.add_partial_value(self, false);
                            parser.values.push(token.into());
                            continue;
                        }
                        self.restore();
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
                            parser.add_value(self);
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
                            parser.add_partial_value(self, true);
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
                    parser.add_attribute_parameter(self);
                    continue;
                }
                b'(' if parser.state != ContentState::AttributeQuotedValue => {
                    if !parser.is_escaped {
                        match parser.state {
                            ContentState::Type
                            | ContentState::AttributeName
                            | ContentState::SubType => {
                                parser.add_attribute(self);
                            }
                            ContentState::AttributeValue => {
                                parser.add_value(self);
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
                        parser.reset_parser();
                    } else {
                        parser.is_escaped = false;
                    }
                    continue;
                }
                b'\r' => continue,
                _ => (),
            }

            if parser.is_escaped {
                parser.is_escaped = false;
            }

            if parser.is_token_start {
                parser.is_token_start = false;
            }

            if parser.token_start == 0 {
                parser.token_start = self.offset();
                parser.token_end = parser.token_start;
            } else {
                parser.token_end = self.offset();
            }
        }

        HeaderValue::Empty
    }
}
#[cfg(test)]
mod tests {
    use crate::parsers::{MessageStream, fields::load_tests};

    #[test]
    fn parse_content_fields() {
        for test in load_tests("content_type.json") {
            assert_eq!(
                MessageStream::new(test.header.as_bytes())
                    .parse_content_type()
                    .into_content_type(),
                test.expected,
                "failed for {:?}",
                test.header
            );
        }

        /*let mut builder = crate::parsers::fields::TestBuilder::new("content_type.json");

        for input in inputs {
            println!("Testing: {:?}", input.0);
            let result = MessageStream::new(input.0.as_bytes())
                .parse_content_type()
                .into_content_type();

            builder.add(input.0.to_string(), result);
        }

        builder.write();*/
    }
}
