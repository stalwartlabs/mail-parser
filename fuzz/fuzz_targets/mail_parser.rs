#![no_main]
use libfuzzer_sys::fuzz_target;

use mail_parser::{
    decoders::{
        base64::Base64Decoder,
        charsets::{
            map::get_charset_decoder,
            single_byte::decoder_iso_8859_1,
            utf::{decoder_utf16, decoder_utf16_be, decoder_utf16_le, decoder_utf7},
        },
        encoded_word::parse_encoded_word,
        hex::decode_hex,
        html::{add_html_token, html_to_text, text_to_html},
        quoted_printable::QuotedPrintableDecoder,
    },
    parsers::{
        fields::{
            address::parse_address,
            content_type::parse_content_type,
            date::parse_date,
            id::parse_id,
            list::parse_comma_separared,
            raw::{parse_and_ignore, parse_raw},
            unstructured::parse_unstructured,
        },
        header::parse_header_name,
        message_stream::MessageStream,
    },
    Message,
};

fuzz_target!(|data: &[u8]| {
    // Fuzz every parsing function
    for n_fuzz in 1..=24 {
        let stream = MessageStream::new(&data);

        match n_fuzz {
            1 => {
                parse_date(&stream, false);
            }
            2 => {
                parse_address(&stream);
            }
            3 => {
                parse_id(&stream);
            }
            4 => {
                parse_comma_separared(&stream);
            }
            5 => {
                parse_and_ignore(&stream);
            }
            6 => {
                parse_raw(&stream);
            }
            7 => {
                parse_unstructured(&stream);
            }
            8 => {
                parse_content_type(&stream);
            }
            9 => {
                parse_header_name(&stream);
            }
            10 => {
                parse_encoded_word(&stream);
            }
            11 => {
                stream.seek_next_part(b"\n");
            }
            12 => {
                stream.get_bytes_to_boundary(b"\n");
            }
            13 => {
                stream.get_bytes_to_boundary(&[]);
            }
            14 => {
                stream.skip_crlf();
            }
            15 => {
                stream.is_boundary_end(0);
            }
            16 => {
                stream.skip_multipart_end();
            }
            17 => {
                stream.decode_base64(b"\n", true);
            }
            18 => {
                stream.decode_base64(b"\n", false);
            }
            19 => {
                stream.decode_base64(&[], true);
            }
            20 => {
                stream.decode_base64(&[], false);
            }
            21 => {
                stream.decode_quoted_printable(b"\n\n", true); // QP Boundaries have to be at least 2 bytes long
            }
            22 => {
                stream.decode_quoted_printable(b"\n\n", false); // QP Boundaries have to be at least 2 bytes long
            }
            23 => {
                stream.decode_quoted_printable(&[], true);
            }
            24 => {
                stream.decode_quoted_printable(&[], false);
            }
            0 | 25..=u32::MAX => unreachable!(),
        }
    }

    // Fuzz HTML functions
    let mut html_str = String::with_capacity(data.len());
    let str_data = String::from_utf8_lossy(data);
    add_html_token(&mut html_str, str_data.as_ref().as_bytes(), false);
    html_to_text(&str_data);
    text_to_html(&str_data);

    // Fuzz decoding functions
    decode_hex(data);
    get_charset_decoder(data);

    let decoders: &[for<'x> fn(&'x [u8]) -> String] = &[
        decoder_utf7,
        decoder_utf16_le,
        decoder_utf16_be,
        decoder_utf16,
        decoder_iso_8859_1,
    ];

    for decoder in decoders {
        decoder(data);
    }

    // Fuzz the entire library
    Message::parse(&data[..]);
});
