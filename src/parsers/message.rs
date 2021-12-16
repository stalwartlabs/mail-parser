/*
 * Copyright Stalwart Labs, Minter Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::borrow::Cow;

use crate::{
    decoders::{
        base64::decode_base64, charsets::map::get_charset_decoder,
        quoted_printable::decode_quoted_printable, DecodeFnc, DecodeResult,
    },
    ContentType, HeaderName, HeaderValue, Message, MessagePart, MessagePartId, MessageStructure,
    Part, RfcHeaders,
};

use super::{
    header::parse_headers,
    mime::{get_bytes_to_boundary, seek_crlf_end, seek_next_part, skip_crlf, skip_multipart_end},
};

#[derive(Debug, PartialEq)]
enum MimeType {
    MultipartMixed,
    MultipartAlernative,
    MultipartRelated,
    MultipartDigest,
    TextPlain,
    TextHtml,
    TextOther,
    Inline,
    Message,
    Other,
}

impl Default for MimeType {
    fn default() -> Self {
        MimeType::Message
    }
}

fn result_to_string<'x>(
    result: DecodeResult,
    data: &'x [u8],
    content_type: Option<&ContentType>,
) -> Cow<'x, str> {
    match (
        result,
        content_type.and_then(|ct| {
            ct.get_attribute("charset")
                .and_then(|c| get_charset_decoder(c.as_bytes()))
        }),
    ) {
        (DecodeResult::Owned(vec), Some(charset_decoder)) => charset_decoder(&vec).into(),
        (DecodeResult::Owned(vec), None) => String::from_utf8(vec)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
            .into(),
        (DecodeResult::Borrowed((from, to)), Some(charset_decoder)) => {
            charset_decoder(&data[from..to]).into()
        }
        (DecodeResult::Borrowed((from, to)), None) => String::from_utf8_lossy(&data[from..to]),
        (DecodeResult::Empty, _) => "\n".to_string().into(),
    }
}

fn result_to_bytes(result: DecodeResult, data: &[u8]) -> Cow<[u8]> {
    match result {
        DecodeResult::Owned(vec) => Cow::Owned(vec),
        DecodeResult::Borrowed((from, to)) => Cow::Borrowed(&data[from..to]),
        DecodeResult::Empty => Cow::from(vec![b'?']),
    }
}

#[inline(always)]
fn get_mime_type(
    content_type: Option<&ContentType>,
    parent_content_type: &MimeType,
) -> (bool, bool, bool, MimeType) {
    if let Some(content_type) = content_type {
        match content_type.get_type() {
            "multipart" => (
                true,
                false,
                false,
                match content_type.get_subtype() {
                    Some("mixed") => MimeType::MultipartMixed,
                    Some("alternative") => MimeType::MultipartAlernative,
                    Some("related") => MimeType::MultipartRelated,
                    Some("digest") => MimeType::MultipartDigest,
                    _ => MimeType::Other,
                },
            ),
            "text" => match content_type.get_subtype() {
                Some("plain") => (false, true, true, MimeType::TextPlain),
                Some("html") => (false, true, true, MimeType::TextHtml),
                _ => (false, false, true, MimeType::TextOther),
            },
            "image" | "audio" | "video" => (false, true, false, MimeType::Inline),
            "message" if content_type.get_subtype() == Some("rfc822") => {
                (false, false, false, MimeType::Message)
            }
            _ => (false, false, false, MimeType::Other),
        }
    } else if let MimeType::MultipartDigest = parent_content_type {
        (false, false, false, MimeType::Message)
    } else {
        (false, true, true, MimeType::TextPlain)
    }
}

#[derive(Default)]
struct MessageParserState {
    mime_type: MimeType,
    mime_boundary: Option<Vec<u8>>,
    in_alternative: bool,
    parts: usize,
    html_parts: usize,
    text_parts: usize,
    need_html_body: bool,
    need_text_body: bool,
    header_id: Option<MessagePartId>,
    structure: Vec<MessageStructure>,
}

impl MessageParserState {
    fn new() -> MessageParserState {
        MessageParserState {
            mime_type: MimeType::Message,
            mime_boundary: None,
            in_alternative: false,
            parts: 0,
            html_parts: 0,
            text_parts: 0,
            need_text_body: true,
            need_html_body: true,
            ..Default::default()
        }
    }

    pub fn get_structure(&mut self) -> MessageStructure {
        if let Some(header_id) = self.header_id {
            MessageStructure::MultiPart((header_id, std::mem::take(&mut self.structure)))
        } else if self.structure.len() > 1 {
            MessageStructure::List(std::mem::take(&mut self.structure))
        } else if self.structure.len() == 1 {
            self.structure.pop().unwrap()
        } else {
            MessageStructure::List(vec![])
        }
    }
}

pub struct MessageStream<'x> {
    pub data: &'x [u8],
    pub pos: usize,
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &'x [u8]) -> MessageStream<'x> {
        MessageStream { data, pos: 0 }
    }
}

impl<'x> Message<'x> {
    fn new() -> Message<'x> {
        Message {
            ..Default::default()
        }
    }

    /// Returns `false` if at least one header field was successfully parsed.
    pub fn is_empty(&self) -> bool {
        self.headers_rfc.is_empty() && self.headers_other.is_empty()
    }

    /// Parses a byte slice containing the RFC5322 raw message and returns a
    /// `Message` struct.
    ///
    /// This function never panics, a best-effort is made to parse the message and
    /// if no headers are found None is returned.
    ///
    pub fn parse(raw_message: &'x [u8]) -> Option<Message<'x>> {
        let mut stream = MessageStream::new(raw_message);

        let mut message = Message::new();
        let mut message_stack = Vec::new();
        let mut message_header_stack = Vec::new();

        let mut state = MessageParserState::new();
        let mut state_stack = Vec::new();

        let mut mime_part_header = RfcHeaders::new();

        message.raw_message = raw_message;

        'outer: loop {
            // Parse headers
            let (is_message, header) = if let MimeType::Message = state.mime_type {
                message.offset_header = stream.pos;
                if !parse_headers(
                    &mut message.headers_rfc,
                    Some(&mut message.headers_other),
                    Some(&mut message.headers_offsets),
                    &mut stream,
                ) {
                    break;
                }
                message.offset_body = seek_crlf_end(&stream, stream.pos);
                (true, &mut message.headers_rfc)
            } else {
                if !parse_headers(&mut mime_part_header, None, None, &mut stream) {
                    break;
                }
                (false, &mut mime_part_header)
            };

            state.parts += 1;

            let content_type = header
                .get(&HeaderName::ContentType)
                .and_then(|c| c.as_content_type_ref());

            let (is_multipart, mut is_inline, mut is_text, mut mime_type) =
                get_mime_type(content_type, &state.mime_type);

            if is_multipart {
                if let Some(mime_boundary) =
                    content_type.map_or_else(|| None, |f| f.get_attribute("boundary"))
                {
                    let mime_boundary = ("\n--".to_string() + mime_boundary).into_bytes();

                    if seek_next_part(&mut stream, mime_boundary.as_ref()) {
                        let new_state = MessageParserState {
                            in_alternative: state.in_alternative
                                || mime_type == MimeType::MultipartAlernative,
                            mime_type,
                            mime_boundary: mime_boundary.into(),
                            html_parts: message.html_body.len(),
                            text_parts: message.text_body.len(),
                            need_html_body: state.need_html_body,
                            need_text_body: state.need_text_body,
                            header_id: if !is_message {
                                Some(message.parts.len())
                            } else {
                                None
                            },
                            ..Default::default()
                        };
                        if !is_message {
                            if mime_part_header.is_empty() {
                                mime_part_header.insert(
                                    HeaderName::ContentType,
                                    HeaderValue::ContentType(ContentType {
                                        c_type: "text".into(),
                                        c_subtype: Some("plain".into()),
                                        attributes: None,
                                    }),
                                );
                            }
                            message.parts.push(MessagePart::Multipart(std::mem::take(
                                &mut mime_part_header,
                            )));
                        }
                        state_stack.push(state);
                        state = new_state;
                        skip_crlf(&mut stream);
                        continue;
                    } else {
                        mime_type = MimeType::TextOther;
                        is_text = true;
                    }
                }
            } else if mime_type == MimeType::Message {
                let new_state = MessageParserState {
                    mime_type: MimeType::Message,
                    mime_boundary: state.mime_boundary.take(),
                    need_html_body: true,
                    need_text_body: true,
                    ..Default::default()
                };
                if mime_part_header.is_empty() {
                    mime_part_header.insert(
                        HeaderName::ContentType,
                        HeaderValue::ContentType(ContentType {
                            c_type: "message".into(),
                            c_subtype: Some("rfc822".into()),
                            attributes: None,
                        }),
                    );
                }
                message_header_stack.push(std::mem::take(&mut mime_part_header));
                message_stack.push(message);
                state_stack.push(state);
                message = Message::new();
                state = new_state;
                skip_crlf(&mut stream);
                continue;
            }

            skip_crlf(&mut stream);

            let (is_binary, decode_fnc): (bool, DecodeFnc) = match header
                .get(&HeaderName::ContentTransferEncoding)
            {
                Some(HeaderValue::Text(encoding)) if encoding.eq_ignore_ascii_case("base64") => {
                    (false, decode_base64)
                }
                Some(HeaderValue::Text(encoding))
                    if encoding.eq_ignore_ascii_case("quoted-printable") =>
                {
                    (false, decode_quoted_printable)
                }
                _ => (true, get_bytes_to_boundary),
            };

            let (bytes_read, mut bytes) = decode_fnc(
                &stream,
                stream.pos,
                state
                    .mime_boundary
                    .as_ref()
                    .map_or_else(|| &[][..], |b| &b[..]),
                false,
            );

            // Attempt to recover contents of an invalid message
            if bytes_read == 0 {
                if stream.pos >= stream.data.len() || (is_binary && state.mime_boundary.is_none()) {
                    break;
                }

                // Get raw MIME part
                let (bytes_read, r_bytes) = if !is_binary {
                    get_bytes_to_boundary(
                        &stream,
                        stream.pos,
                        state
                            .mime_boundary
                            .as_ref()
                            .map_or_else(|| &[][..], |b| &b[..]),
                        false,
                    )
                } else {
                    (0, DecodeResult::Empty)
                };

                if bytes_read == 0 {
                    // If there is MIME boundary, ignore it and get raw message
                    if state.mime_boundary.is_some() {
                        let (bytes_read, r_bytes) =
                            get_bytes_to_boundary(&stream, stream.pos, &[][..], false);
                        if bytes_read > 0 {
                            bytes = r_bytes;
                            stream.pos += bytes_read;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    bytes = r_bytes;
                    stream.pos += bytes_read;
                }
                mime_type = MimeType::TextOther;
                is_inline = false;
                is_text = true;
            } else {
                stream.pos += bytes_read;
            }

            let is_inline = is_inline
                && header
                    .get(&HeaderName::ContentDisposition)
                    .map_or_else(|| true, |d| !d.get_content_type().is_attachment())
                && (state.parts == 1
                    || (state.mime_type != MimeType::MultipartRelated
                        && (mime_type == MimeType::Inline
                            || content_type.map_or_else(|| true, |c| !c.has_attribute("name")))));

            let (add_to_html, add_to_text) = if let MimeType::MultipartAlernative = state.mime_type
            {
                match mime_type {
                    MimeType::TextHtml => (true, false),
                    MimeType::TextPlain => (false, true),
                    _ => (false, false),
                }
            } else if is_inline {
                if state.in_alternative && (state.need_text_body || state.need_html_body) {
                    match mime_type {
                        MimeType::TextHtml => {
                            state.need_text_body = false;
                        }
                        MimeType::TextPlain => {
                            state.need_html_body = false;
                        }
                        _ => (),
                    }
                }
                (state.need_html_body, state.need_text_body)
            } else {
                (false, false)
            };

            if is_text {
                let is_html = mime_type == MimeType::TextHtml;
                let text_part = Part {
                    body: result_to_string(bytes, stream.data, content_type),
                    headers: std::mem::take(&mut mime_part_header),
                };

                if add_to_html && !is_html {
                    message.html_body.push(message.parts.len());
                } else if add_to_text && is_html {
                    message.text_body.push(message.parts.len());
                }

                if add_to_html && is_html {
                    message.html_body.push(message.parts.len());
                } else if add_to_text && !is_html {
                    message.text_body.push(message.parts.len());
                } else {
                    message.attachments.push(message.parts.len());
                }
                state
                    .structure
                    .push(MessageStructure::Part(message.parts.len()));
                message.parts.push(if is_html {
                    MessagePart::Html(text_part)
                } else {
                    MessagePart::Text(text_part)
                });
            } else {
                if add_to_html {
                    message.html_body.push(message.parts.len());
                }
                if add_to_text {
                    message.text_body.push(message.parts.len());
                }

                message.attachments.push(message.parts.len());

                state
                    .structure
                    .push(MessageStructure::Part(message.parts.len()));

                let binary_part = Part::new(
                    std::mem::take(&mut mime_part_header),
                    result_to_bytes(bytes, stream.data),
                );

                message.parts.push(if !is_inline {
                    MessagePart::Binary(binary_part)
                } else {
                    MessagePart::InlineBinary(binary_part)
                });
            };

            if state.mime_boundary.is_some() {
                // Currently processing a MIME part
                let mut last_part_offset = stream.pos;

                'inner: loop {
                    if let MimeType::Message = state.mime_type {
                        // Finished processing nested message, restore parent message from stack
                        if let (Some(mut prev_message), Some(mut prev_state)) =
                            (message_stack.pop(), state_stack.pop())
                        {
                            message.structure = state.get_structure();
                            message.offset_end = seek_crlf_end(&stream, last_part_offset);
                            message.raw_message =
                                &raw_message[message.offset_header..message.offset_end];
                            prev_message.attachments.push(prev_message.parts.len());
                            prev_message.parts.push(MessagePart::Message(Part::new(
                                message_header_stack.pop().unwrap(),
                                Box::new(message),
                            )));
                            message = prev_message;
                            prev_state.mime_boundary = state.mime_boundary;
                            state = prev_state;
                        } else {
                            debug_assert!(false, "Failed to restore parent message. Aborting.");
                            break 'outer;
                        }
                    }

                    if skip_multipart_end(&mut stream) {
                        // End of MIME part reached

                        if MimeType::MultipartAlernative == state.mime_type
                            && state.need_html_body
                            && state.need_text_body
                        {
                            // Found HTML part only
                            if state.text_parts == message.text_body.len()
                                && state.html_parts != message.html_body.len()
                            {
                                for &part_id in &message.html_body[state.html_parts..] {
                                    message.text_body.push(part_id);
                                }
                            }

                            // Found text part only
                            if state.html_parts == message.html_body.len()
                                && state.text_parts != message.text_body.len()
                            {
                                for &part_id in &message.text_body[state.html_parts..] {
                                    message.html_body.push(part_id);
                                }
                            }
                        }

                        if let Some(mut prev_state) = state_stack.pop() {
                            // Add headers and substructure to parent part
                            prev_state.structure.push(state.get_structure());

                            // Restore ancestor's state
                            state = prev_state;
                            last_part_offset = stream.pos;

                            if let Some(ref mime_boundary) = state.mime_boundary {
                                // Ancestor has a MIME boundary, seek it.
                                if seek_next_part(&mut stream, mime_boundary) {
                                    continue 'inner;
                                }
                            }
                        }
                        break 'outer;
                    } else {
                        skip_crlf(&mut stream);
                        // Headers of next part expected next, break inner look.
                        break 'inner;
                    }
                }
            } else if stream.pos >= stream.data.len() {
                break 'outer;
            }
        }

        while let (Some(mut prev_message), Some(prev_state)) =
            (message_stack.pop(), state_stack.pop())
        {
            message.offset_end = stream.pos;
            if !message.is_empty() {
                message.structure = state.get_structure();
                prev_message.attachments.push(prev_message.parts.len());
                prev_message.parts.push(MessagePart::Message(Part::new(
                    message_header_stack.pop().unwrap(),
                    Box::new(message),
                )));
            }
            message = prev_message;
            state = prev_state;
        }

        message.structure = state.get_structure();
        message.offset_end = stream.pos;

        if !message.is_empty() {
            Some(message)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{parsers::message::Message, MessagePart};

    const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----\n".as_bytes();

    #[test]
    fn parse_full_messages() {
        for test_suite in ["rfc", "legacy", "thirdparty", "malformed"] {
            let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            test_dir.push("tests");
            test_dir.push(test_suite);

            let mut tests_run = 0;

            for file_name in fs::read_dir(&test_dir).unwrap() {
                let file_name = file_name.as_ref().unwrap().path();
                if file_name.extension().map_or(false, |e| e == "txt") {
                    let mut input = fs::read(&file_name).unwrap();
                    let mut pos = 0;

                    for sep_pos in 0..input.len() {
                        if input[sep_pos..sep_pos + SEPARATOR.len()].eq(SEPARATOR) {
                            pos = sep_pos;
                            break;
                        }
                    }

                    assert!(
                        pos > 0,
                        "Failed to find separator in test file '{}'.",
                        file_name.display()
                    );

                    tests_run += 1;

                    let input = input.split_at_mut(pos);
                    let mut message = Message::parse(input.0).unwrap();

                    // Remove raw message references before comparing
                    message.raw_message = &[];
                    for parts in &mut message.parts {
                        if let MessagePart::Message(part) = parts {
                            part.body.raw_message = &[];
                            for parts in &mut part.body.parts {
                                if let MessagePart::Message(part) = parts {
                                    part.body.raw_message = &[];
                                }
                            }
                        }
                    }

                    assert_eq!(
                        message,
                        serde_json::from_slice::<Message>(&input.1[SEPARATOR.len()..]).unwrap(),
                        "Test failed for '{}', result was:\n{}",
                        file_name.display(),
                        serde_json::to_string_pretty(&message).unwrap()
                    );
                }
            }

            assert!(
                tests_run > 0,
                "Did not find any tests to run in folder {}.",
                test_dir.display()
            );
        }
    }

    /*#[test]
    fn message_to_yaml() {
        let mut file_name = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file_name.push("tests");
        file_name.push("rfc");
        file_name.push("007.txt");

        let mut input = fs::read(&file_name).unwrap();
        let mut pos = 0;

        for sep_pos in 0..input.len() {
            if input[sep_pos..sep_pos + SEPARATOR.len()].eq(SEPARATOR) {
                pos = sep_pos;
                break;
            }
        }

        assert!(
            pos > 0,
            "Failed to find separator in test file '{}'.",
            file_name.display()
        );

        let input = input.split_at_mut(pos).0;
        let message = Message::parse(input).unwrap();
        let result = serde_yaml::to_string(&message).unwrap();

        fs::write("test.yaml", &result).unwrap();

        println!("{:}", result);
    }

    #[test]
    fn generate_test_samples() {
        const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----\n".as_bytes();

        for test_suite in ["legacy", "malformed", "rfc", "thirdparty"] {
            let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            test_dir.push("tests");
            test_dir.push(test_suite);

            for file_name in fs::read_dir(test_dir).unwrap() {
                if file_name
                    .as_ref()
                    .unwrap()
                    .path()
                    .to_str()
                    .unwrap()
                    .contains("COPYING")
                {
                    continue;
                }

                let mut input = fs::read(file_name.as_ref().unwrap().path()).unwrap();
                let mut pos = 0;
                for sep_pos in 0..input.len() {
                    if input[sep_pos..sep_pos + SEPARATOR.len()].eq(SEPARATOR) {
                        pos = sep_pos;
                        break;
                    }
                }
                assert!(pos > 0, "Failed to find separator.");
                let input = input.split_at_mut(pos);

                /*println!(
                    "{}",
                    serde_json::to_string_pretty(&Message::parse(input.0)).unwrap()
                );*/

                let mut output = Vec::new();
                output.extend_from_slice(input.0);
                output.extend_from_slice(SEPARATOR);
                output.extend_from_slice(
                    serde_json::to_string_pretty(&Message::parse(input.0).unwrap())
                        .unwrap()
                        .as_bytes(),
                );
                fs::write(file_name.as_ref().unwrap().path(), &output).unwrap();
            }
        }
    }*/
}
