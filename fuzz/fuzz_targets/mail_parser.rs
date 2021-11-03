#![no_main]
use libfuzzer_sys::fuzz_target;

use mail_parser::{
    decoders::{
        base64::decode_base64,
        charsets::{
            map::get_charset_decoder,
            single_byte::decoder_iso_8859_1,
            utf::{decoder_utf16, decoder_utf16_be, decoder_utf16_le, decoder_utf7},
        },
        encoded_word::decode_rfc2047,
        hex::decode_hex,
        html::{add_html_token, html_to_text, text_to_html},
        quoted_printable::decode_quoted_printable,
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
        mime::*,
        header::parse_header_name,
        message::MessageStream,
    },
    Message,
};

fuzz_target!(|data: &[u8]| {
    // Fuzz every parsing function
    for n_fuzz in 1..=24 {
        let mut stream = MessageStream::new(&data);

        match n_fuzz {
            1 => {
                parse_date(&mut stream);
            }
            2 => {
                parse_address(&mut stream);
            }
            3 => {
                parse_id(&mut stream);
            }
            4 => {
                parse_comma_separared(&mut stream);
            }
            5 => {
                parse_and_ignore(&mut stream);
            }
            6 => {
                parse_raw(&mut stream);
            }
            7 => {
                parse_unstructured(&mut stream);
            }
            8 => {
                parse_content_type(&mut stream);
            }
            9 => {
                parse_header_name(&mut stream);
            }
            10 => {
                decode_rfc2047(&mut stream, 0);
            }
            11 => {
                seek_next_part(&mut stream, b"\n");
            }
            12 => {
                get_bytes_to_boundary(&mut stream, 0, b"\n", false);
            }
            13 => {
                get_bytes_to_boundary(&mut stream, 0, &[], false);
            }
            14 => {
                skip_crlf(&mut stream);
            }
            15 => {
                is_boundary_end(&mut stream, 0);
            }
            16 => {
                skip_multipart_end(&mut stream);
            }
            17 => {
                decode_base64(&mut stream, 0, b"\n", true);
            }
            18 => {
                decode_base64(&mut stream, 0, b"\n", false);
            }
            19 => {
                decode_base64(&mut stream, 0, &[], true);
            }
            20 => {
                decode_base64(&mut stream, 0, &[], false);
            }
            21 => {
                decode_quoted_printable(&mut stream, 0, b"\n\n", true); // QP Boundaries have to be at least 2 bytes long
            }
            22 => {
                decode_quoted_printable(&mut stream, 0, b"\n\n", false); // QP Boundaries have to be at least 2 bytes long
            }
            23 => {
                decode_quoted_printable(&mut stream, 0, &[], true);
            }
            24 => {
                decode_quoted_printable(&mut stream, 0, &[], false);
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
