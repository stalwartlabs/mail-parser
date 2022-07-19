/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level dir&ectory of this distribution.
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
    ContentType, HeaderValue, Message, MessageAttachment, MessagePart, MessagePartId, PartType,
    RawHeaders, RfcHeader, RfcHeaders,
};

use super::{
    header::parse_headers,
    mime::{get_bytes_to_boundary, seek_crlf, seek_next_part, skip_crlf, skip_multipart_end},
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
            "message" if [Some("rfc822"), Some("global")].contains(&content_type.get_subtype()) => {
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

#[inline(always)]
fn add_missing_type<'x>(
    headers: &mut RfcHeaders<'x>,
    c_type: Cow<'x, str>,
    c_subtype: Cow<'x, str>,
) {
    if headers.is_empty() {
        headers.insert(
            RfcHeader::ContentType,
            HeaderValue::ContentType(ContentType {
                c_type,
                c_subtype: Some(c_subtype),
                attributes: None,
            }),
        );
    }
}

#[derive(Default, Debug)]
struct MessageParserState {
    mime_type: MimeType,
    mime_boundary: Option<Vec<u8>>,
    in_alternative: bool,
    parts: usize,
    html_parts: usize,
    text_parts: usize,
    need_html_body: bool,
    need_text_body: bool,
    part_id: MessagePartId,
    sub_part_ids: Vec<MessagePartId>,
    offset_header: usize,
    offset_body: usize,
    offset_end: usize,
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
        self.parts.is_empty()
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

        let mut state = MessageParserState::new();
        let mut state_stack = Vec::with_capacity(4);

        let mut part_header = RfcHeaders::new();
        let mut part_header_raw = RawHeaders::new();

        'outer: loop {
            // Parse headers
            state.offset_header = stream.pos;
            if !parse_headers(&mut part_header, &mut part_header_raw, &mut stream) {
                break;
            }

            state.parts += 1;
            state.sub_part_ids.push(message.parts.len());

            let content_type = part_header
                .get(&RfcHeader::ContentType)
                .and_then(|c| c.as_content_type_ref());

            let (is_multipart, mut is_inline, mut is_text, mut mime_type) =
                get_mime_type(content_type, &state.mime_type);

            if is_multipart {
                if let Some(mime_boundary) =
                    content_type.map_or_else(|| None, |f| f.get_attribute("boundary"))
                {
                    //let mime_boundary = format!("\n--{}", mime_boundary).into_bytes();
                    let mime_boundary = format!("--{}", mime_boundary).into_bytes();
                    state.offset_body = seek_crlf(&stream, stream.pos);

                    if seek_next_part(&mut stream, mime_boundary.as_ref()) {
                        let part_id = message.parts.len();
                        let new_state = MessageParserState {
                            in_alternative: state.in_alternative
                                || mime_type == MimeType::MultipartAlernative,
                            mime_type,
                            mime_boundary: mime_boundary.into(),
                            html_parts: message.html_body.len(),
                            text_parts: message.text_body.len(),
                            need_html_body: state.need_html_body,
                            need_text_body: state.need_text_body,
                            part_id,
                            ..Default::default()
                        };
                        add_missing_type(&mut part_header, "text".into(), "plain".into());
                        message.parts.push(MessagePart {
                            headers_rfc: std::mem::take(&mut part_header),
                            headers_raw: std::mem::take(&mut part_header_raw),
                            offset_header: state.offset_header,
                            offset_body: state.offset_body,
                            offset_end: 0,
                            is_encoding_problem: false,
                            body: PartType::default(),
                        });
                        state_stack.push((state, None));
                        state = new_state;
                        skip_crlf(&mut stream);
                        continue;
                    } else {
                        mime_type = MimeType::TextOther;
                        is_text = true;
                    }
                }
            }

            skip_crlf(&mut stream);
            state.offset_body = stream.pos;

            let (is_binary, decode_fnc): (bool, DecodeFnc) = match part_header
                .get(&RfcHeader::ContentTransferEncoding)
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

            if is_binary && mime_type == MimeType::Message {
                let new_state = MessageParserState {
                    mime_type: MimeType::Message,
                    mime_boundary: state.mime_boundary.take(),
                    need_html_body: true,
                    need_text_body: true,
                    part_id: message.parts.len(),
                    ..Default::default()
                };
                add_missing_type(&mut part_header, "message".into(), "rfc822".into());
                message.attachments.push(message.parts.len());
                message.parts.push(MessagePart {
                    headers_rfc: std::mem::take(&mut part_header),
                    headers_raw: std::mem::take(&mut part_header_raw),
                    is_encoding_problem: false,
                    offset_header: state.offset_header,
                    offset_body: state.offset_body,
                    offset_end: 0,
                    body: PartType::default(), // Temp value, will be replaced later.
                });
                state_stack.push((state, message.into()));
                message = Message::new();
                state = new_state;
                continue;
            }

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
            let is_encoding_problem = bytes_read == 0;
            if is_encoding_problem {
                let did_recover = if !(stream.pos >= stream.data.len()
                    || (is_binary && state.mime_boundary.is_none()))
                {
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
                        // If there is a MIME boundary, ignore it and get raw message
                        if state.mime_boundary.is_some() {
                            let (bytes_read, r_bytes) =
                                get_bytes_to_boundary(&stream, stream.pos, &[][..], false);
                            if bytes_read > 0 {
                                bytes = r_bytes;
                                stream.pos += bytes_read;
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        bytes = r_bytes;
                        stream.pos += bytes_read;
                        true
                    }
                } else {
                    false
                };

                if did_recover {
                    mime_type = MimeType::TextOther;
                    is_inline = false;
                    is_text = true;
                } else {
                    // Could not recover error, add part and abort
                    message.parts.push(MessagePart {
                        headers_rfc: std::mem::take(&mut part_header),
                        headers_raw: std::mem::take(&mut part_header_raw),
                        is_encoding_problem: true,
                        body: PartType::Binary((&[][..]).into()),
                        offset_header: state.offset_header,
                        offset_body: state.offset_body,
                        offset_end: stream.pos,
                    });
                    break;
                }
            } else {
                stream.pos += bytes_read;
            }
            state.offset_end = state
                .mime_boundary
                .as_ref()
                .map(|b| stream.pos.saturating_sub(b.len()))
                .unwrap_or(stream.pos);

            let body_part = if mime_type != MimeType::Message {
                let is_inline = is_inline
                    && part_header
                        .get(&RfcHeader::ContentDisposition)
                        .map_or_else(|| true, |d| !d.get_content_type().is_attachment())
                    && (state.parts == 1
                        || (state.mime_type != MimeType::MultipartRelated
                            && (mime_type == MimeType::Inline
                                || content_type
                                    .map_or_else(|| true, |c| !c.has_attribute("name")))));

                let (add_to_html, add_to_text) =
                    if let MimeType::MultipartAlernative = state.mime_type {
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
                    let text = result_to_string(bytes, stream.data, content_type);
                    let is_html = mime_type == MimeType::TextHtml;

                    add_missing_type(&mut part_header, "text".into(), "plain".into());

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

                    if is_html {
                        PartType::Html(text)
                    } else {
                        PartType::Text(text)
                    }
                } else {
                    if add_to_html {
                        message.html_body.push(message.parts.len());
                    }
                    if add_to_text {
                        message.text_body.push(message.parts.len());
                    }

                    message.attachments.push(message.parts.len());

                    let bytes = result_to_bytes(bytes, stream.data);
                    if !is_inline {
                        PartType::Binary(bytes)
                    } else {
                        PartType::InlineBinary(bytes)
                    }
                }
            } else {
                add_missing_type(&mut part_header, "message".into(), "rfc822".into());
                message.attachments.push(message.parts.len());
                PartType::Message(MessageAttachment::Raw(result_to_bytes(bytes, stream.data)))
            };

            // Add part
            message.parts.push(MessagePart {
                headers_rfc: std::mem::take(&mut part_header),
                headers_raw: std::mem::take(&mut part_header_raw),
                is_encoding_problem,
                body: body_part,
                offset_header: state.offset_header,
                offset_body: state.offset_body,
                offset_end: state.offset_end,
            });

            if state.mime_boundary.is_some() {
                // Currently processing a MIME part
                'inner: loop {
                    if let MimeType::Message = state.mime_type {
                        // Finished processing a nested message, restore parent message from stack
                        if let Some((mut prev_state, Some(mut prev_message))) = state_stack.pop() {
                            let offset_end = state
                                .mime_boundary
                                .as_ref()
                                .map(|b| stream.pos.saturating_sub(b.len()))
                                .unwrap_or(stream.pos);
                            message.raw_message =
                                raw_message[state.offset_header..offset_end].as_ref().into();

                            if let Some(part) = prev_message.parts.get_mut(state.part_id) {
                                part.body =
                                    PartType::Message(MessageAttachment::Parsed(Box::new(message)));
                                part.offset_end = offset_end;
                            } else {
                                debug_assert!(false, "Invalid part ID, could not find message.");
                            }

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

                        if let Some(part) = message.parts.get_mut(state.part_id) {
                            // Update end offset
                            part.offset_end = seek_crlf(&stream, stream.pos);
                            // Add headers and substructure to parent part
                            part.body =
                                PartType::Multipart(std::mem::take(&mut state.sub_part_ids));
                        } else {
                            debug_assert!(false, "Invalid part ID, could not find multipart.");
                        }

                        if let Some((prev_state, _)) = state_stack.pop() {
                            // Restore ancestor's state
                            state = prev_state;

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

        // Corrupted MIME message, try to recover whatever is possible.
        while let Some((prev_state, prev_message)) = state_stack.pop() {
            if let Some(mut prev_message) = prev_message {
                message.raw_message = raw_message[state.offset_header..stream.pos].as_ref().into();

                if let Some(part) = prev_message.parts.get_mut(state.part_id) {
                    part.body = PartType::Message(MessageAttachment::Parsed(Box::new(message)));
                    part.offset_end = stream.pos;
                } else {
                    debug_assert!(false, "Invalid part ID, could not find message.");
                }

                message = prev_message;
            } else if let Some(part) = message.parts.get_mut(state.part_id) {
                part.offset_end = stream.pos;
                part.body = PartType::Multipart(state.sub_part_ids);
            } else {
                debug_assert!(false, "This should not have happened.");
            }
            state = prev_state;
        }

        message.raw_message = raw_message.into();

        if !message.is_empty() {
            Some(message)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, path::PathBuf};

    use serde::Serialize;
    use serde_json::Value;

    use crate::{
        parsers::message::Message, HeaderValue, MessagePart, MessagePartId, PartType, RawHeaders,
        RfcHeader,
    };

    const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----".as_bytes();

    #[derive(Serialize)]
    pub struct SortedMessage<'x> {
        pub html_body: Vec<MessagePartId>,
        pub text_body: Vec<MessagePartId>,
        pub attachments: Vec<MessagePartId>,
        pub parts: Vec<SortedMessagePart<'x>>,
    }

    #[derive(Serialize)]
    pub struct SortedMessagePart<'x> {
        pub headers_rfc: BTreeMap<RfcHeader, HeaderValue<'x>>,
        pub headers_raw: RawHeaders<'x>,
        pub is_encoding_problem: bool,
        pub body: PartType<'x>,
        pub offset_header: usize,
        pub offset_body: usize,
        pub offset_end: usize,
    }

    impl<'x> From<Message<'x>> for SortedMessage<'x> {
        fn from(message: Message<'x>) -> Self {
            SortedMessage {
                html_body: message.html_body,
                text_body: message.text_body,
                attachments: message.attachments,
                parts: message.parts.into_iter().map(|p| p.into()).collect(),
            }
        }
    }

    impl<'x> From<MessagePart<'x>> for SortedMessagePart<'x> {
        fn from(part: MessagePart<'x>) -> Self {
            SortedMessagePart {
                headers_rfc: part.headers_rfc.into_iter().collect(),
                headers_raw: part.headers_raw,
                is_encoding_problem: part.is_encoding_problem,
                body: part.body,
                offset_header: part.offset_header,
                offset_body: part.offset_body,
                offset_end: part.offset_end,
            }
        }
    }

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

                    let (raw_message, expected_result) = input.split_at_mut(pos);
                    let message = SortedMessage::from(Message::parse(raw_message).unwrap());
                    let json_message = serde_json::to_string_pretty(&message).unwrap();

                    assert_eq!(
                        serde_json::from_str::<Value>(&json_message).unwrap(),
                        serde_json::from_str::<Value>(
                            std::str::from_utf8(&expected_result[SEPARATOR.len()..]).unwrap()
                        )
                        .unwrap(),
                        "Test failed for '{}', result was:\n{}",
                        file_name.display(),
                        json_message
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
        file_name.push("003.txt");

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
        const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----".as_bytes();

        for test_suite in [/*"legacy", "malformed", "rfc",*/ "thirdparty"] {
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

                /*if !file_name
                    .as_ref()
                    .unwrap()
                    .path()
                    .to_str()
                    .unwrap()
                    .contains("011")
                {
                    continue;
                }*/

                println!("{:}", file_name.as_ref().unwrap().path().display());
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
                    serde_json::to_string_pretty(&SortedMessage::from(
                        Message::parse(input.0).unwrap(),
                    ))
                    .unwrap()
                    .as_bytes(),
                );
                fs::write(file_name.as_ref().unwrap().path(), &output).unwrap();
            }
        }
    }*/
}
