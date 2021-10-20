use std::borrow::{Borrow, Cow};

use crate::decoders::{
    base64::Base64Decoder,
    binary::BinaryDecoder,
    buffer_writer::BufferWriter,
    charsets::map::{get_charset_decoder, get_default_decoder},
    quoted_printable::QuotedPrintableDecoder,
    Writer,
};

use super::{
    fields::{content_type::ContentType, MessageField},
    header::{self, parse_headers, MessageHeader, MimeHeader},
    message_stream::MessageStream,
};

#[derive(Debug, Default)]
pub struct Message<'x> {
    header: MessageHeader<'x>,
    html_body: Vec<TextPart<'x>>,
    text_body: Vec<TextPart<'x>>,
    attachments: Vec<MessageAttachment<'x>>,
}

#[derive(Debug, Default)]
pub struct TextPart<'x> {
    header: MimeHeader<'x>,
    contents: Cow<'x, str>,
}

#[derive(Debug, Default)]
pub struct BinaryPart<'x> {
    header: MimeHeader<'x>,
    contents: Box<[u8]>,
}

#[derive(Debug)]
pub enum MessageAttachment<'x> {
    Text(Box<TextPart<'x>>),
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
) -> (bool, bool, MimeType) {
    if let Some(content_type) = content_type {
        match content_type.get_type() {
            "multipart" => (
                true,
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
                Some("plain") => (false, true, MimeType::TextPlain),
                Some("html") => (false, true, MimeType::TextHtml),
                _ => (false, false, MimeType::TextOther),
            },
            "image" | "audio" | "video" => (false, true, MimeType::Inline),
            "message" if content_type.get_subtype() == Some("rfc822") => {
                (false, false, MimeType::Message)
            }
            _ => (false, false, MimeType::Other),
        }
    } else if let MimeType::MultipartDigest = parent_content_type {
        (false, false, MimeType::Message)
    } else {
        (false, true, MimeType::TextPlain)
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

pub fn parse_message<'x>(stream: &'x MessageStream<'x>) -> Message<'x> {
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
        if !parse_headers(header, stream) {
            // EOF found while parsing headers, abort.
            println!("EOF during parse headers");
            break;
        }

        state.parts += 1;

        let (is_multipart, is_inline, mime_type) =
            get_mime_type(header.get_content_type(), &state.mime_type);

        println!(
            "--- New part {:?} parent {:?} at '{:?}'",
            mime_type,
            state.mime_type,
            stream
                .get_string(stream.get_pos(), stream.get_pos() + 50, true)
                .unwrap_or_else(|| "NOTHING".into())
        );

        if is_multipart {
            if let Some(mime_boundary) = header
                .get_content_type()
                .map_or_else(|| None, |f| f.get_attribute("boundary"))
            {
                println!("Found boundary '{}'", mime_boundary,);
                let mime_boundary = ("\n--".to_string() + mime_boundary).into_bytes();

                if stream.seek_bytes(mime_boundary.as_ref()) {
                    println!(
                        "Seek to '{:?}'",
                        stream
                            .get_string(stream.get_pos(), stream.get_pos() + 50, true)
                            .unwrap_or_else(|| "NOTHING".into())
                    );
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
                    println!("Seek bytes failed!");
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

        let (mut writer, is_text): (Box<dyn Writer>, bool) =
            if let MimeType::TextHtml | MimeType::TextPlain | MimeType::TextOther = mime_type {
                (
                    if let Some(charset) = header
                        .get_content_type()
                        .map_or_else(|| None, |c| c.get_attribute("charset"))
                    {
                        get_charset_decoder(charset.as_bytes(), 200)
                            .unwrap_or_else(|| get_default_decoder(200))
                    } else {
                        get_default_decoder(200)
                    },
                    true,
                )
            } else {
                (Box::new(BufferWriter::with_capacity(200)), false)
            };

        if !(match header.get_content_transfer_encoding() {
            Some(encoding) if encoding.eq_ignore_ascii_case("base64") => stream.decode_base64(
                state
                    .mime_boundary
                    .as_ref()
                    .map_or_else(|| &[][..], |b| &b[..]),
                false,
                writer.as_mut(),
            ),
            Some(encoding) if encoding.eq_ignore_ascii_case("quoted-printable") => stream
                .decode_quoted_printable(
                    state
                        .mime_boundary
                        .as_ref()
                        .map_or_else(|| &[][..], |b| &b[..]),
                    false,
                    writer.as_mut(),
                ),
            _ => stream.decode_binary(
                state
                    .mime_boundary
                    .as_ref()
                    .map_or_else(|| &[][..], |b| &b[..]),
                writer.as_mut(),
            ),
        }) && (state.mime_boundary.is_none()
            || !stream.seek_bytes(state.mime_boundary.as_ref().unwrap()))
        {
            println!("Failed to parse encoded part. Aborting.");
            break;
        }

        // TODO return Cow<[u8]> when straight from mail

        if writer.is_empty() {
            writer.write_byte(&b'?');
            if let Some(ref m) = state.mime_boundary {
                println!("Empty part!! '{}'", std::str::from_utf8(m).unwrap());
            } else {
                println!("Empty part!!");
            }
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
                header: std::mem::take(&mut mime_part_header),
                contents: writer.get_string().unwrap().into(),
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
                    header: std::mem::take(&mut mime_part_header),
                    contents: writer.get_string().unwrap().into(),
                }))
            } else {
                MessageAttachment::File(Box::new(BinaryPart {
                    header: std::mem::take(&mut mime_part_header),
                    contents: writer.get_bytes().unwrap(),
                }))
            });
        }

        if state.mime_boundary.is_some() {
            // Currently processing a MIME part

            'inner: loop {
                if let MimeType::Message = state.mime_type {
                    // Finished processing nested message, restore parent message from stack
                    println!("Finished processing message!");
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
                        println!("Failed to restore parent message!");
                        break 'outer;
                    }
                }

                if stream.skip_bytes("--".as_bytes()) {
                    // End of MIME part reached
                    println!(
                        "Mime part end '{:?}--' next '{:?}'",
                        std::str::from_utf8(state.mime_boundary.as_ref().unwrap().as_ref())
                            .unwrap(),
                        stream
                            .get_string(stream.get_pos(), stream.get_pos() + 10, true)
                            .unwrap_or_else(|| "NOTHING".into())
                    );

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
                                    header: MimeHeader::new(),
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
                                    header: MimeHeader::new(),
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
                    println!("Expecting headers of next part.");
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
    use std::path::PathBuf;

    use crate::parsers::message_stream::MessageStream;

    use super::parse_message;

    #[test]
    fn body_parse() {
        let inputs = [(
            concat!(
                "Subject: This is a test email\n",
                "Content-Type: multipart/alternative; boundary=foobar\n",
                "Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)\n",
                "Message-Id: <1038776827.1181.6.camel@hurina>\n",
                "List-Id: Dovecot Mailing List <dovecot.procontrol.fi>\n",
                "List-Unsubscribe: <http://procontrol.fi/cgi-bin/mailman/listinfo/dovecot>,\n",
                "    <mailto:dovecot-request@procontrol.fi?subject=unsubscribe>\n",
                "List-Archive: <http://procontrol.fi/pipermail/dovecot>\n",
                "List-Post: <mailto:dovecot@procontrol.fi>\n",
                "List-Help: <mailto:dovecot-request@procontrol.fi?subject=help>\n",
                "List-Subscribe: <http://procontrol.fi/cgi-bin/mailman/listinfo/dovecot>,\n",
                "    <mailto:dovecot-request@procontrol.fi?subject=subscribe>\n",
                "\n",
                "--foobar\n",
                "Content-Type: text/plain; charset=utf-8\n",
                "Content-Transfer-Encoding: quoted-printable\n",
                "\n",
                "This is the plaintext version, in utf-8. Proof by Euro: =E2=82=AC\n",
                "--foobar\n",
                "Content-Type: text/html\n",
                "Content-Transfer-Encoding: base64\n",
                "\n",
                "PGh0bWw+PGJvZHk+VGhpcyBpcyB0aGUgPGI+SFRNTDwvYj4gdmVyc2lvbiwgaW4g \n",
                "dXMtYXNjaWkuIFByb29mIGJ5IEV1cm86ICZldXJvOzwvYm9keT48L2h0bWw+Cg== \n",
                "--foobar--\n",
                "After the final boundary stuff gets ignored.\n"
            ),
            "",
        )];

        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        //d.push("test/mbox/multipart-complex.eml");
        //d.push("test/mbox/multipart-complex.eml");
        d.push("test/samples/messages/m2014.txt");

        let input = std::fs::read(d).unwrap();

        let stream = MessageStream::new(&input);
        let message = parse_message(&stream);
        println!("Text: {:?}\n\n{}", message.text_body, "-".repeat(50));
        println!("Html: {:?}\n\n{}", message.html_body, "-".repeat(50));
        println!("Attach: {:?}\n\n{}", message.attachments, "-".repeat(50));
    }
}
