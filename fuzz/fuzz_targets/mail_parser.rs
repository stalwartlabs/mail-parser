#![no_main]
use libfuzzer_sys::fuzz_target;

use mail_parser::{
    decoders::{
        base64::base64_decode,
        charsets::map::charset_decoder,
        hex::decode_hex,
        html::{add_html_token, html_to_text, text_to_html},
        quoted_printable::quoted_printable_decode,
    },
    parsers::{
        fields::thread::{thread_name, trim_trailing_fwd},
        MessageStream,
    },
    Message, MessageParser,
};

static RFC822_ALPHABET: &[u8] = b"0123456789abcdefghijklm:=- \r\n";

fuzz_target!(|data: &[u8]| {
    for data_ in [
        std::borrow::Cow::from(data),
        std::borrow::Cow::from(into_alphabet(data, RFC822_ALPHABET)),
    ] {
        let data = data_.as_ref();

        // Fuzz every parsing function
        MessageStream::new(data).parse_date();
        MessageStream::new(data).parse_address();
        MessageStream::new(data).parse_id();
        MessageStream::new(data).parse_comma_separared();
        MessageStream::new(data).parse_and_ignore();
        MessageStream::new(data).parse_raw();
        MessageStream::new(data).parse_unstructured();
        MessageStream::new(data).parse_content_type();
        MessageStream::new(data).parse_headers(&MessageParser::default(), &mut Vec::new());
        MessageStream::new(data).parse_header_name();
        MessageStream::new(data).decode_rfc2047();

        MessageStream::new(data).seek_next_part(b"\n");
        MessageStream::new(data).mime_part(b"\n");
        MessageStream::new(data).seek_part_end(b"\n"[..].into());
        MessageStream::new(data).skip_crlf();
        MessageStream::new(data).is_multipart_end();

        base64_decode(data);
        MessageStream::new(data).decode_base64_word();
        MessageStream::new(data).decode_base64_mime(b"\n");
        quoted_printable_decode(data);
        MessageStream::new(data).decode_quoted_printable_word();
        MessageStream::new(data).decode_quoted_printable_mime(b"\n");

        // Fuzz text functions
        let mut html_str = String::with_capacity(data.len());
        let str_data = String::from_utf8_lossy(data);
        add_html_token(&mut html_str, str_data.as_ref().as_bytes(), false);
        html_to_text(&str_data);
        text_to_html(&str_data);
        thread_name(&str_data);
        trim_trailing_fwd(&str_data);

        // Fuzz decoding functions
        decode_hex(data);
        charset_decoder(data);

        // Fuzz the entire library
        MessageParser::default().parse(data);
    }
});

fn into_alphabet(data: &[u8], alphabet: &[u8]) -> Vec<u8> {
    data.iter()
        .map(|&byte| alphabet[byte as usize % alphabet.len()])
        .collect()
}
