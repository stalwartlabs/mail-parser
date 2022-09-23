#![no_main]
use libfuzzer_sys::fuzz_target;

use mail_parser::{
    decoders::{
        base64::{decode_base64, decode_base64_mime, decode_base64_word},
        charsets::{
            map::get_charset_decoder,
            single_byte::decoder_iso_8859_1,
            utf::{decoder_utf16, decoder_utf16_be, decoder_utf16_le, decoder_utf7},
        },
        encoded_word::decode_rfc2047,
        hex::decode_hex,
        html::{add_html_token, html_to_text, text_to_html},
        quoted_printable::{
            decode_quoted_printable, decode_quoted_printable_mime, decode_quoted_printable_word,
        },
    },
    parsers::{
        fields::{
            address::parse_address,
            content_type::parse_content_type,
            date::parse_date,
            id::parse_id,
            list::parse_comma_separared,
            raw::{parse_and_ignore, parse_raw},
            thread::{thread_name, trim_trailing_fwd},
            unstructured::parse_unstructured,
        },
        header::parse_header_name,
        message::MessageStream,
        mime::*,
    },
    Message,
};

static RFC822_ALPHABET: &[u8] = b"0123456789abcdefghijklm:=- \r\n";

fuzz_target!(|data: &[u8]| {
    for data_ in [
        std::borrow::Cow::from(data),
        std::borrow::Cow::from(into_alphabet(data, RFC822_ALPHABET)),
    ] {
        let data = data_.as_ref();

        // Fuzz every parsing function
        parse_date(&mut MessageStream::new(&data));
        parse_address(&mut MessageStream::new(&data));
        parse_id(&mut MessageStream::new(&data));
        parse_comma_separared(&mut MessageStream::new(&data));
        parse_and_ignore(&mut MessageStream::new(&data));
        parse_raw(&mut MessageStream::new(&data));
        parse_unstructured(&mut MessageStream::new(&data));
        parse_content_type(&mut MessageStream::new(&data));
        parse_header_name(&data);
        decode_rfc2047(&mut MessageStream::new(&data), 0);

        seek_next_part(&mut MessageStream::new(&data), b"\n");
        get_mime_part(&mut MessageStream::new(&data), b"\n");
        seek_part_end(&mut MessageStream::new(&data), b"\n"[..].into());
        skip_crlf(&mut MessageStream::new(&data));
        skip_multipart_end(&mut MessageStream::new(&data));

        decode_base64(&data);
        decode_base64_word(&data);
        decode_base64_mime(&mut MessageStream::new(&data), b"\n");
        decode_quoted_printable(&data);
        decode_quoted_printable_word(&data);
        decode_quoted_printable_mime(&mut MessageStream::new(&data), b"\n");

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
        get_charset_decoder(data);

        for decoder in &[
            decoder_utf7,
            decoder_utf16_le,
            decoder_utf16_be,
            decoder_utf16,
            decoder_iso_8859_1,
        ] as &[for<'x> fn(&'x [u8]) -> String]
        {
            decoder(data);
        }

        // Fuzz the entire library
        Message::parse(&data[..]);
    }
});

fn into_alphabet(data: &[u8], alphabet: &[u8]) -> Vec<u8> {
    data.iter()
        .map(|&byte| alphabet[byte as usize % alphabet.len()])
        .collect()
}
