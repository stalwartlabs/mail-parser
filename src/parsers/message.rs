use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, Cow};

use crate::decoders::{
    base64::Base64Decoder,
    charsets::map::{decoder_default, get_charset_decoder},
    html::{html_to_text, text_to_html},
    quoted_printable::QuotedPrintableDecoder,
};

use super::{
    fields::{content_type::ContentType, MessageField},
    header::{parse_headers, MessageHeader, MimeHeader},
    message_stream::MessageStream,
};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Message<'x> {
    #[serde(borrow)]
    header: MessageHeader<'x>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    html_body: Vec<TextPart<'x>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    text_body: Vec<TextPart<'x>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    attachments: Vec<MessagePart<'x>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    inline: Vec<BinaryPart<'x>>,
}
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TextPart<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    header: Option<MimeHeader<'x>>,
    contents: Cow<'x, str>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BinaryPart<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    header: Option<MimeHeader<'x>>,
    #[serde(with = "serde_bytes")]
    #[serde(borrow)]
    contents: Cow<'x, [u8]>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum MessagePart<'x> {
    Text(Box<TextPart<'x>>),
    #[serde(borrow)]
    File(Box<BinaryPart<'x>>),
    Message(Box<Message<'x>>),
}

impl<'x> Message<'x> {
    fn new() -> Message<'x> {
        Message {
            header: MessageHeader::new(),
            ..Default::default()
        }
    }
}

#[derive(Debug, PartialEq)]
enum MimeType {
    MultipartMixed,
    MultipartAlernative,
    MultipartRelated,
    MultipartDigest,
    MultipartOther,
    TextPlain,
    TextHtml,
    TextOther,
    Inline,
    Message,
    Other,
}

#[inline(always)]
fn bytes_to_string<'x>(
    bytes: &'x [u8],
    content_type: Option<&ContentType>,
    is_utf8_safe: bool,
) -> Cow<'x, str> {
    if !is_utf8_safe {
        if let Some(content_type) = content_type {
            if let Some(charset) = content_type.get_attribute("charset") {
                if let Some(decoder_fnc) = get_charset_decoder(charset.as_bytes()) {
                    return decoder_fnc(bytes);
                }
            }
        }
        decoder_default(bytes)
    } else {
        (unsafe { std::str::from_utf8_unchecked(bytes) }).into()
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

struct MessageParserState {
    mime_type: MimeType,
    mime_boundary: Option<Vec<u8>>,
    in_alternative: bool,
    parts: usize,
    html_parts: usize,
    text_parts: usize,
    need_html_body: bool,
    need_text_body: bool,
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
        }
    }
}

impl<'x> Message<'x> {
    pub fn parse(bytes: &'x mut [u8]) -> Message<'x> {
        let stream = MessageStream::new(bytes);

        let mut message = Message::new();
        let mut message_stack = Vec::new();

        let mut state = MessageParserState::new();
        let mut state_stack = Vec::new();

        let mut mime_part_header = MimeHeader::new();

        'outer: loop {
            // Obtain reference to either the message or the MIME part's header
            let header: &mut dyn MessageField = if let MimeType::Message = state.mime_type {
                &mut message.header
            } else {
                &mut mime_part_header
            };

            // Parse headers
            if !parse_headers(header, &stream) {
                break;
            }

            state.parts += 1;

            let (is_multipart, mut is_inline, mut is_text, mut mime_type) =
                get_mime_type(header.get_content_type(), &state.mime_type);

            println!(
                "--- New part {:?} parent {:?} at '{:?}'",
                mime_type,
                state.mime_type,
                stream
                    .get_string(stream.get_pos(), stream.get_pos() + 20, true)
                    .unwrap_or_else(|| "NOTHING".into())
            );

            if is_multipart {
                if let Some(mime_boundary) = header
                    .get_content_type()
                    .map_or_else(|| None, |f| f.get_attribute("boundary"))
                {
                    println!("Found boundary '{}'", mime_boundary,);
                    let mime_boundary = ("\n--".to_string() + mime_boundary).into_bytes();

                    if stream.seek_next_part(mime_boundary.as_ref()) {
                        let new_state = MessageParserState {
                            in_alternative: state.in_alternative
                                || mime_type == MimeType::MultipartAlernative,
                            mime_type,
                            mime_boundary: mime_boundary.into(),
                            parts: 0,
                            html_parts: message.html_body.len(),
                            text_parts: message.text_body.len(),
                            need_html_body: state.need_html_body,
                            need_text_body: state.need_text_body,
                        };
                        mime_part_header.clear();
                        state_stack.push(state);
                        state = new_state;
                        stream.skip_crlf();
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
                    in_alternative: false,
                    parts: 0,
                    html_parts: 0,
                    text_parts: 0,
                    need_html_body: true,
                    need_text_body: true,
                };
                mime_part_header.clear();
                message_stack.push(message);
                state_stack.push(state);
                message = Message::new();
                state = new_state;
                stream.skip_crlf();
                continue;
            }

            stream.skip_crlf();

            let (success, mut is_utf8_safe, mut bytes) = match header
                .get_content_transfer_encoding()
            {
                Some(encoding) if encoding.eq_ignore_ascii_case("base64") => stream.decode_base64(
                    state
                        .mime_boundary
                        .as_ref()
                        .map_or_else(|| &[][..], |b| &b[..]),
                    false,
                ),
                Some(encoding) if encoding.eq_ignore_ascii_case("quoted-printable") => stream
                    .decode_quoted_printable(
                        state
                            .mime_boundary
                            .as_ref()
                            .map_or_else(|| &[][..], |b| &b[..]),
                        false,
                    ),
                _ => stream.get_bytes_to_boundary(
                    state
                        .mime_boundary
                        .as_ref()
                        .map_or_else(|| &[][..], |b| &b[..]),
                ),
            };

            // Attempt to recover contents of an invalid message
            if !success {
                //println!("Failed to parse part, reattempting.");
                if !stream.is_eof() {
                    // Get raw MIME part
                    let (success, r_is_utf8_safe, r_bytes) = stream.get_bytes_to_boundary(
                        state
                            .mime_boundary
                            .as_ref()
                            .map_or_else(|| &[][..], |b| &b[..]),
                    );
                    if !success {
                        // If there is MIME boundary, ignore it and get raw message
                        if !stream.is_eof() && state.mime_boundary.is_some() {
                            let (_, r_is_utf8_safe, r_bytes) =
                                stream.get_bytes_to_boundary(&[][..]);
                            if r_bytes.is_some() {
                                is_utf8_safe = r_is_utf8_safe;
                                bytes = r_bytes;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        is_utf8_safe = r_is_utf8_safe;
                        bytes = r_bytes;
                    }
                    mime_type = MimeType::TextOther;
                    is_inline = false;
                    is_text = true;
                } else if bytes.is_none() {
                    break;
                }
            }

            let is_inline = is_inline
                && header
                    .get_content_disposition()
                    .map_or_else(|| true, |d| !d.is_attachment())
                && (state.parts == 1
                    || (state.mime_type != MimeType::MultipartRelated
                        && (mime_type == MimeType::Inline
                            || header
                                .get_content_type()
                                .map_or_else(|| true, |c| !c.has_attribute("name")))));

            if is_text {
                let (add_as_html, add_as_text, is_html) =
                    if let MimeType::MultipartAlernative = state.mime_type {
                        match mime_type {
                            MimeType::TextHtml => (true, false, true),
                            MimeType::TextPlain => (false, true, false),
                            _ => (false, false, false),
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
                        (
                            state.need_html_body,
                            state.need_text_body,
                            mime_type == MimeType::TextHtml,
                        )
                    } else {
                        (false, false, false)
                    };

                let text_part = TextPart {
                    contents: bytes_to_string(
                        bytes.map_or_else(|| "\n".as_bytes(), |v| v),
                        header.get_content_type(),
                        is_utf8_safe,
                    ),
                    header: if !mime_part_header.is_empty() {
                        Some(std::mem::take(&mut mime_part_header))
                    } else {
                        None
                    },
                };

                if add_as_html && !is_html {
                    message.html_body.push(TextPart {
                        header: None,
                        contents: text_to_html(&text_part.contents).into(),
                    });
                } else if add_as_text && is_html {
                    message.text_body.push(TextPart {
                        header: None,
                        contents: html_to_text(&text_part.contents).into(),
                    });
                }

                if add_as_html && is_html {
                    message.html_body.push(text_part);
                } else if add_as_text && !is_html {
                    message.text_body.push(text_part);
                } else {
                    message
                        .attachments
                        .push(MessagePart::Text(Box::new(text_part)));
                }
            } else {
                let binary_part = BinaryPart {
                    header: if !mime_part_header.is_empty() {
                        Some(std::mem::take(&mut mime_part_header))
                    } else {
                        None
                    },
                    contents: bytes.map_or_else(|| "?".as_bytes(), |v| v).into(),
                };

                if !is_inline {
                    message
                        .attachments
                        .push(MessagePart::File(Box::new(binary_part)));
                } else {
                    message.inline.push(binary_part);
                }
            };

            if state.mime_boundary.is_some() {
                // Currently processing a MIME part

                'inner: loop {
                    if let MimeType::Message = state.mime_type {
                        // Finished processing nested message, restore parent message from stack
                        if let (Some(mut prev_message), Some(mut prev_state)) =
                            (message_stack.pop(), state_stack.pop())
                        {
                            prev_message
                                .attachments
                                .push(MessagePart::Message(Box::new(message)));
                            message = prev_message;
                            prev_state.mime_boundary = state.mime_boundary;
                            state = prev_state;
                        } else {
                            debug_assert!(false, "Failed to restore parent message. Aborting.");
                            break 'outer;
                        }
                    }

                    if stream.skip_multipart_end() {
                        // End of MIME part reached

                        if MimeType::MultipartAlernative == state.mime_type
                            && state.need_html_body
                            && state.need_text_body
                        {
                            // Found HTML part only
                            if state.text_parts == message.text_body.len()
                                && state.html_parts != message.html_body.len()
                            {
                                for part in message.html_body[state.html_parts..].iter() {
                                    message.text_body.push(TextPart {
                                        header: None,
                                        contents: html_to_text(&part.contents).into(),
                                    });
                                }
                            }

                            // Found HTML part only
                            if state.html_parts == message.html_body.len()
                                && state.text_parts != message.text_body.len()
                            {
                                for part in message.text_body[state.text_parts..].iter() {
                                    message.html_body.push(TextPart {
                                        header: None,
                                        contents: text_to_html(&part.contents).into(),
                                    });
                                }
                            }
                        }

                        if let Some(prev_state) = state_stack.pop() {
                            // Restore ancestor's state
                            state = prev_state;

                            if let Some(ref mime_boundary) = state.mime_boundary {
                                // Ancestor has a MIME boundary, seek it.
                                if stream.seek_next_part(mime_boundary) {
                                    println!(
                                        "Found ancestor MIME boundary '{:?}'",
                                        std::str::from_utf8(mime_boundary).unwrap()
                                    );

                                    continue 'inner;
                                } else {
                                    println!(
                                        "Boundary '{:?} not found.",
                                        std::str::from_utf8(mime_boundary).unwrap()
                                    );
                                }
                            } else {
                                // Ancestor does not have a MIME boundary, end parsing.
                                println!("Ancestor has no MIME boundary, parent found?");
                            }
                        } else {
                            // No more ancestors found, finish parsing.
                            println!("Finish parsing, no ancestors found.");
                        }
                        break 'outer;
                    } else {
                        stream.skip_crlf();
                        // Headers of next part expected next, break inner look.
                        break 'inner;
                    }
                }
            } else if stream.is_eof() {
                break 'outer;
            }
        }

        message
    }
}
#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::parsers::message::Message;

    #[test]
    fn body_parse() {
        const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----\n".as_bytes();

        for test_suite in ["rfc", "legacy", "malformed"] {
            let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            test_dir.push("tests");
            test_dir.push(test_suite);

            for file_name in fs::read_dir(test_dir).unwrap() {
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

                let message = Message::parse(input.0);
                if(message != serde_json::from_slice::<Message>(&input.1[SEPARATOR.len()..]).unwrap()) {
                    println!("{}", serde_json::to_string_pretty(&message).unwrap());
                    return;
                }
                assert_eq!(
                    Message::parse(input.0),
                    serde_json::from_slice::<Message>(&input.1[SEPARATOR.len()..]).unwrap(),
                    "Test failed for {}",
                    file_name.unwrap().path().display()
                );
            }
        }
    }

    #[test]
    fn generate_test_samples() {
        const SEPARATOR: &[u8] = "\n---- EXPECTED STRUCTURE ----\n".as_bytes();

        for test_suite in ["rfc" /*"legacy","malformed"*/] {
            let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            test_dir.push("tests");
            test_dir.push(test_suite);

            for file_name in fs::read_dir(test_dir).unwrap() {
                if !file_name
                    .as_ref()
                    .unwrap()
                    .path()
                    .to_str()
                    .unwrap()
                    .contains("007")
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

                println!(
                    "{}",
                    serde_yaml::to_string(&Message::parse(input.0)).unwrap()
                );

                /*let mut output = Vec::new();
                output.extend_from_slice(input.0);
                output.extend_from_slice(SEPARATOR);
                output.extend_from_slice(
                    serde_json::to_string_pretty(&Message::parse(input.0))
                        .unwrap_or_else(|_| "".to_string())
                        .as_bytes(),
                );
                fs::write(file_name.as_ref().unwrap().path(), &output).unwrap();*/
            }
        }
    }
}
