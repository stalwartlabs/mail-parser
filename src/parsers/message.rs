use std::borrow::{Borrow, Cow};
use serde::{Serialize, Deserialize};

use crate::decoders::{
    base64::Base64Decoder, buffer_writer::BufferWriter, bytes::BytesDecoder,
    charsets::map::get_charset_decoder, quoted_printable::QuotedPrintableDecoder, Writer,
};

use super::{
    fields::{content_type::ContentType, MessageField},
    header::{parse_headers, MessageHeader, MimeHeader},
    message_stream::MessageStream,
};

#[derive(Debug, Default, Serialize, Deserialize)]
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
    attachments: Vec<MessageAttachment<'x>>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TextPart<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    header: Option<MimeHeader<'x>>,
    contents: Cow<'x, str>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BinaryPart<'x> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    header: Option<MimeHeader<'x>>,
    contents: &'x [u8],
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageAttachment<'x> {
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

pub fn parse_message<'x>(stream: &'x MessageStream<'x>, buffer: &'x BufferWriter) -> Message<'x> {
    let mut message = Message::new();
    let mut state = MessageParserState::new();
    let mut state_stack = Vec::new();
    let mut message_stack = Vec::new();
    let mut mime_part_header = MimeHeader::new();

    'outer: loop {
        // Obtain reference to either the message or the MIME part's header
        let header: &mut dyn MessageField = if let MimeType::Message = state.mime_type {
            &mut message.header
        } else {
            &mut mime_part_header
        };

        // Parse headers
        if !parse_headers(header, stream, buffer) {
            // EOF found while parsing headers, abort.
            debug_assert!(false, "EOF found while parsing header. Aborting.");
            break;
        }

        state.parts += 1;

        let (is_multipart, is_inline, is_text, mime_type) =
            get_mime_type(header.get_content_type(), &state.mime_type);

        /*println!(
            "--- New part {:?} parent {:?} at '{:?}'",
            mime_type,
            state.mime_type,
            stream
                .get_string(stream.get_pos(), stream.get_pos() + 50, true)
                .unwrap_or_else(|| "NOTHING".into())
        );*/

        if is_multipart {
            if let Some(mime_boundary) = header
                .get_content_type()
                .map_or_else(|| None, |f| f.get_attribute("boundary"))
            {
                //println!("Found boundary '{}'", mime_boundary,);
                let mime_boundary = ("\n--".to_string() + mime_boundary).into_bytes();

                if stream.seek_bytes(mime_boundary.as_ref()) {
                    /*println!(
                        "Seek to '{:?}'",
                        stream
                            .get_string(stream.get_pos(), stream.get_pos() + 50, true)
                            .unwrap_or_else(|| "NOTHING".into())
                    );*/
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
                    debug_assert!(false, "MIME boundary seek failed. Aborting.");
                    break;
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

        let decoder: Option<Box<dyn Writer>> = if let Some(charset) = header
            .get_content_type()
            .map_or_else(|| None, |c| c.get_attribute("charset"))
        {
            get_charset_decoder(charset.as_bytes(), buffer)
        } else {
            None
        };

        let (success, mut is_utf8_safe, mut bytes) = match header.get_content_transfer_encoding() {
            Some(encoding) if encoding.eq_ignore_ascii_case("base64") => (
                stream.decode_base64(
                    state
                        .mime_boundary
                        .as_ref()
                        .map_or_else(|| &[][..], |b| &b[..]),
                    false,
                    decoder
                        .as_ref()
                        .map_or_else(|| buffer as &dyn Writer, |f| f.as_ref()),
                ),
                decoder.is_some(),
                buffer.get_bytes(),
            ),
            Some(encoding) if encoding.eq_ignore_ascii_case("quoted-printable") => (
                stream.decode_quoted_printable(
                    state
                        .mime_boundary
                        .as_ref()
                        .map_or_else(|| &[][..], |b| &b[..]),
                    false,
                    decoder
                        .as_ref()
                        .map_or_else(|| buffer as &dyn Writer, |f| f.as_ref()),
                ),
                decoder.is_some(),
                buffer.get_bytes(),
            ),
            _ => {
                if let Some(decoder) = decoder {
                    (
                        stream.decode_bytes(
                            state
                                .mime_boundary
                                .as_ref()
                                .map_or_else(|| &[][..], |b| &b[..]),
                            decoder.as_ref(),
                        ),
                        true,
                        buffer.get_bytes(),
                    )
                } else {
                    stream.get_raw_bytes(
                        state
                            .mime_boundary
                            .as_ref()
                            .map_or_else(|| &[][..], |b| &b[..]),
                    )
                }
            }
        };

        if !success {
            if !stream.is_eof() {
                let (success, r_is_utf8_safe, r_bytes) = stream.get_raw_bytes(
                    state
                        .mime_boundary
                        .as_ref()
                        .map_or_else(|| &[][..], |b| &b[..]),
                );
                if !success {
                    debug_assert!(false, "Failed to parse encoded part. Aborting.");
                    break;
                }
                is_utf8_safe = r_is_utf8_safe;
                bytes = r_bytes;
            } else {
                debug_assert!(false, "Failed to parse encoded part. Aborting.");
                break;
            }
        }

        if bytes.is_none() {
            bytes = Some("?".as_bytes());
            /*if let Some(ref m) = state.mime_boundary {
                println!("Empty part!! '{}'", std::str::from_utf8(m).unwrap());
            } else {
                println!("Empty part!!");
            }*/
        }

        if is_inline
            && is_text
            && header
                .get_content_disposition()
                .map_or_else(|| true, |d| !d.is_attachment())
            && (state.parts == 1
                || (state.mime_type != MimeType::MultipartRelated
                    && (mime_type == MimeType::Inline
                        || header
                            .get_content_type()
                            .map_or_else(|| true, |c| !c.has_attribute("name")))))
        {
            let text_part = TextPart {
                header: if !mime_part_header.is_empty() {
                    Some(std::mem::take(&mut mime_part_header))
                } else {
                    None
                },
                contents: if is_utf8_safe {
                    unsafe { std::str::from_utf8_unchecked(bytes.unwrap()).into() }
                } else {
                    String::from_utf8_lossy(bytes.unwrap())
                },
            };

            let is_this_alternative = if let MimeType::MultipartAlernative = state.mime_type {
                true
            } else if state.in_alternative && (state.need_text_body || state.need_html_body) {
                match mime_type {
                    MimeType::TextHtml => {
                        state.need_text_body = false;
                    }
                    MimeType::TextPlain => {
                        state.need_html_body = false;
                    }
                    _ => (),
                }
                false
            } else {
                false
            };

            match mime_type {
                MimeType::TextHtml if is_this_alternative || state.need_html_body => {
                    message.html_body.push(text_part)
                }
                MimeType::TextPlain if is_this_alternative || state.need_text_body => {
                    message.text_body.push(text_part)
                }
                _ => message
                    .attachments
                    .push(MessageAttachment::Text(Box::new(text_part))),
            }
        } else {
            message.attachments.push(if is_text {
                MessageAttachment::Text(Box::new(TextPart {
                    header: if !mime_part_header.is_empty() {
                        Some(std::mem::take(&mut mime_part_header))
                    } else {
                        None
                    },
                    contents: if is_utf8_safe {
                        unsafe { std::str::from_utf8_unchecked(bytes.unwrap()).into() }
                    } else {
                        String::from_utf8_lossy(bytes.unwrap())
                    },
                }))
            } else {
                MessageAttachment::File(Box::new(BinaryPart {
                    header: if !mime_part_header.is_empty() {
                        Some(std::mem::take(&mut mime_part_header))
                    } else {
                        None
                    },
                    contents: bytes.unwrap(),
                }))
            });
        }

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
                            .push(MessageAttachment::Message(Box::new(message)));
                        message = prev_message;
                        prev_state.mime_boundary = state.mime_boundary;
                        state = prev_state;
                    } else {
                        debug_assert!(false, "Failed to restore parent message. Aborting.");
                        break 'outer;
                    }
                }

                if stream.skip_bytes("--".as_bytes()) {
                    // End of MIME part reached
                    /*println!(
                        "Mime part end '{:?}--' next '{:?}'",
                        std::str::from_utf8(state.mime_boundary.as_ref().unwrap().as_ref())
                            .unwrap(),
                        stream
                            .get_string(stream.get_pos(), stream.get_pos() + 10, true)
                            .unwrap_or_else(|| "NOTHING".into())
                    );*/

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
                                    contents: part.contents.clone(), //Todo make plain text
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
                                    contents: part.contents.replace("\n", "<br/>").into(),
                                });
                            }
                        }
                    }

                    if let Some(prev_state) = state_stack.pop() {
                        // Restore ancestor's state
                        state = prev_state;

                        if let Some(ref mime_boundary) = state.mime_boundary {
                            // Ancestor has a MIME boundary, seek it.
                            if stream.seek_bytes(mime_boundary) {
                                // Boundary not found, probably a corrupted message, abort.
                                /*println!(
                                    "Found ancestor MIME boundary '{:?}'",
                                    std::str::from_utf8(mime_boundary).unwrap()
                                );*/
                                continue 'inner;
                            } /*else {
                                println!(
                                    "Boundary '{:?} not found.",
                                    std::str::from_utf8(mime_boundary).unwrap()
                                );
                            }*/
                        } /*else {
                            // Ancestor does not have a MIME boundary, end parsing.
                            println!("Ancestor has no MIME boundary, parent found?");
                        } */
                    } /*else {
                        // No more ancestors found, finish parsing.
                        println!("Finish parsing, no ancestors found.");
                    } */
                    break 'outer;
                } else {
                    stream.skip_crlf();
                    // Headers of next part expected next, break inner look.
                    //println!("Expecting headers of next part.");
                    break 'inner;
                }
            }
        } else if stream.is_eof() {
            break 'outer;
        }
    }

    message
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{decoders::buffer_writer::BufferWriter, parsers::message_stream::MessageStream};

    use super::parse_message;

    #[test]
    fn body_parse() {
        let mut samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        samples_dir.push("test/dovecot");
        let mut got_sign = false;
        let mut count = 0;
        
        for path in fs::read_dir(samples_dir.to_str().unwrap()).unwrap() {
            let mut path = path.unwrap().path();
            if path.as_path().to_str().unwrap().contains(".yaml") {
                continue;
            }
            //if !path.as_path().to_str().unwrap().contains("m576") {
            //    continue;
            //}            
            println!("Parsing {}", path.to_str().unwrap());
            let input = fs::read(path.as_path()).unwrap();
            if String::from_utf8_lossy(&input).contains("application/pgp-signature") {
                if !got_sign {
                    got_sign = true;
                } else {
                    continue;
                }
            }

            let stream = MessageStream::new(&input);
            let buffer = BufferWriter::with_capacity((input.len() as f64 * 1.25) as usize);
            let message = parse_message(&stream, &buffer);
            
            if message.text_body.len() + message.html_body.len() + message.attachments.len() > 1 {
                fs::write(format!("/vagrant/Code/stalwart/test/final/list_{:03}.eml", count), &input).unwrap();
                //assert!(path.set_extension("yaml"));
                fs::write(format!("/vagrant/Code/stalwart/test/final/list_{:03}.yaml", count), serde_yaml::to_string(&message).unwrap()).unwrap();
                //fs::write(path, serde_yaml::to_string(&message).unwrap()).unwrap();
                count += 1;
            }

        }
    }
}
