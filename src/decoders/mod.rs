/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Decoders the parser uses that are also useful on their own: charset
//! conversion, HTML to text and text to HTML, and body previews. Transfer
//! encodings (base64, quoted-printable, hex) are decoded by encodify; use
//! it directly for those.

pub mod charsets;
pub mod html;
mod prefix;
pub mod preview;

use crate::Encoding;
use charsets::Charset;
use encodify::{base64, qp};
use std::borrow::Cow;

pub(crate) use prefix::{Limit, TextPrefix, text_prefix};

const KNOWN_TRANSFER_ENCODINGS: [&[u8]; 5] =
    [b"7bit", b"8bit", b"binary", b"base64", b"quoted-printable"];

pub(crate) fn transfer_decode(bytes: &[u8], encoding: Encoding) -> Cow<'_, [u8]> {
    match encoding {
        Encoding::None => Cow::Borrowed(bytes),
        Encoding::QuotedPrintable => qp::BODY.decode(bytes).unwrap_or(Cow::Borrowed(bytes)),
        Encoding::Base64 => {
            let mut out = Vec::new();
            transfer_decode_append(bytes, encoding, &mut out);
            Cow::Owned(out)
        }
    }
}

pub(crate) fn transfer_decode_checked(bytes: &[u8], encoding: Encoding) -> (Cow<'_, [u8]>, bool) {
    match encoding {
        Encoding::None => (Cow::Borrowed(bytes), false),
        Encoding::QuotedPrintable => match qp::BODY.strict().decode(bytes) {
            Ok(decoded) => (decoded, false),
            Err(_) => (transfer_decode(bytes, encoding), true),
        },
        Encoding::Base64 => {
            let mut out = Vec::new();
            let malformed = base64::MIME.decode_append(bytes, &mut out).is_err();
            if malformed {
                base64_tolerant(bytes, &mut out);
            }
            (Cow::Owned(out), malformed)
        }
    }
}

pub(crate) fn is_known_transfer_encoding(value: &[u8]) -> bool {
    let token = value
        .split(|&byte| byte == b'(')
        .next()
        .unwrap_or_default()
        .trim_ascii();
    KNOWN_TRANSFER_ENCODINGS
        .iter()
        .any(|known| token.eq_ignore_ascii_case(known))
}

pub(crate) fn transfer_decode_append(bytes: &[u8], encoding: Encoding, out: &mut Vec<u8>) {
    match encoding {
        Encoding::None => out.extend_from_slice(bytes),
        Encoding::QuotedPrintable => {
            if qp::BODY.decode_append(bytes, out).is_err() {
                out.extend_from_slice(bytes);
            }
        }
        Encoding::Base64 => {
            if base64::MIME.decode_append(bytes, out).is_err() {
                base64_tolerant(bytes, out);
            }
        }
    }
}

pub(crate) fn transfer_decoded_len(bytes: &[u8], encoding: Encoding) -> usize {
    match encoding {
        Encoding::None => bytes.len(),
        Encoding::Base64 => base64::MIME.decoded_len(bytes).unwrap_or_else(|_| {
            let mut out = Vec::new();
            base64_tolerant(bytes, &mut out);
            out.len()
        }),
        Encoding::QuotedPrintable => qp::BODY.decoded_len(bytes).unwrap_or(bytes.len()),
    }
}

fn base64_tolerant(rest: &[u8], out: &mut Vec<u8>) {
    base64_feed(rest, out).finish(out);
}

fn base64_feed(mut rest: &[u8], out: &mut Vec<u8>) -> base64::Decoder {
    let mut decoder = base64::MIME.decoder();
    loop {
        rest = rest
            .get(decoder.decode_symbols(rest, out)..)
            .unwrap_or_default();
        match rest.split_first() {
            None => return decoder,
            Some((b'=', tail)) => {
                decoder.pad(out);
                rest = tail;
            }
            Some((_, tail)) => rest = tail,
        }
    }
}

pub(crate) fn charset(label: Option<&str>) -> Charset {
    label
        .and_then(|label| Charset::from_label(label.as_bytes()))
        .unwrap_or_default()
}

pub(crate) fn rewrite<'x>(
    text: Cow<'x, str>,
    rewrite: impl for<'a> FnOnce(&'a str) -> Cow<'a, str>,
) -> Cow<'x, str> {
    match text {
        Cow::Borrowed(text) => rewrite(text),
        Cow::Owned(text) => {
            let rewritten = match rewrite(&text) {
                Cow::Owned(rewritten) => Some(rewritten),
                Cow::Borrowed(same) if same.len() == text.len() => None,
                Cow::Borrowed(other) => Some(other.to_owned()),
            };
            Cow::Owned(rewritten.unwrap_or(text))
        }
    }
}

pub(crate) fn to_text<'x>(bytes: Cow<'x, [u8]>, charset: Charset) -> Cow<'x, str> {
    match bytes {
        Cow::Borrowed(bytes) => charset.decode(bytes),
        Cow::Owned(bytes) => Cow::Owned(charset.decode_owned(bytes)),
    }
}

pub(crate) fn to_text_checked<'x>(bytes: Cow<'x, [u8]>, charset: Charset) -> (Cow<'x, str>, bool) {
    match bytes {
        Cow::Borrowed(bytes) => charset.decode_checked(bytes),
        Cow::Owned(bytes) => {
            let (text, malformed) = charset.decode_owned_checked(bytes);
            (Cow::Owned(text), malformed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::tests::Rng;

    #[test]
    fn qp_matches_codec() {
        let alphabet = b"ab \t\r\n=\xc3\xa9";
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for _ in 0..20_000 {
            let len = rng.below(24);
            let input: Vec<u8> = (0..len)
                .map(|_| {
                    alphabet
                        .get(rng.below(alphabet.len()))
                        .copied()
                        .unwrap_or(b'a')
                })
                .collect();
            let mut decoded = Vec::new();
            qp::BODY
                .decode_append(&input, &mut decoded)
                .expect("never fails");
            let borrowed = transfer_decode(&input, Encoding::QuotedPrintable);
            assert_eq!(borrowed.as_ref(), decoded.as_slice(), "{input:?}");
            if matches!(borrowed, Cow::Borrowed(_)) {
                assert_eq!(decoded, input);
            }
            assert_eq!(
                transfer_decoded_len(&input, Encoding::QuotedPrintable),
                decoded.len()
            );
        }
        assert!(matches!(
            transfer_decode(b"plain text\r\nline two\nend", Encoding::QuotedPrintable),
            Cow::Borrowed(_)
        ));
        assert!(matches!(
            transfer_decode(b"trailing \r\n", Encoding::QuotedPrintable),
            Cow::Owned(_)
        ));
    }

    #[test]
    fn base64_best_effort() {
        assert_eq!(
            transfer_decode(b"SGVs\r\nbG8=", Encoding::Base64).as_ref(),
            b"Hello"
        );
        assert_eq!(
            transfer_decode(b"SGVs-bG8=", Encoding::Base64).as_ref(),
            b"Hello"
        );
        assert_eq!(
            transfer_decode(b"VGVzdA", Encoding::Base64).as_ref(),
            b"Test"
        );
        assert_eq!(transfer_decoded_len(b"SGVs-bG8=", Encoding::Base64), 5);
        assert_eq!(transfer_decoded_len(b"SGVsbG8=", Encoding::Base64), 5);
    }

    #[test]
    fn checked_transfer_decoding() {
        let alphabet = b"ab \t\r\n=\xc3\xa9AZ09+/-";
        let mut rng = Rng(0x7ab5_c0de_1234_9876);
        for _ in 0..20_000 {
            let len = rng.below(24);
            let input: Vec<u8> = (0..len)
                .map(|_| {
                    alphabet
                        .get(rng.below(alphabet.len()))
                        .copied()
                        .unwrap_or(b'a')
                })
                .collect();
            for encoding in [Encoding::None, Encoding::QuotedPrintable, Encoding::Base64] {
                let (checked, malformed) = transfer_decode_checked(&input, encoding);
                assert_eq!(checked, transfer_decode(&input, encoding), "{input:?}");
                let expected = match encoding {
                    Encoding::None => false,
                    Encoding::QuotedPrintable => qp::BODY.strict().decoded_len(&input).is_err(),
                    Encoding::Base64 => base64::MIME.decoded_len(&input).is_err(),
                };
                assert_eq!(malformed, expected, "{encoding:?} {input:?}");
            }
        }
        for (input, encoding, decoded, malformed) in [
            (&b"SGVs\r\nbG8="[..], Encoding::Base64, &b"Hello"[..], false),
            (b"SGVs-bG8=", Encoding::Base64, b"Hello", true),
            (b"VGVzdA", Encoding::Base64, b"Test", false),
            (
                b"caf=C3=A9",
                Encoding::QuotedPrintable,
                b"caf\xc3\xa9",
                false,
            ),
            (
                b"soft=\r\nbreak",
                Encoding::QuotedPrintable,
                b"softbreak",
                false,
            ),
            (b"a=ZZb", Encoding::QuotedPrintable, b"a=ZZb", true),
            (b"end=4", Encoding::QuotedPrintable, b"end=4", true),
            (b"as is=", Encoding::None, b"as is=", false),
        ] {
            let (checked, problem) = transfer_decode_checked(input, encoding);
            assert_eq!(
                (checked.as_ref(), problem),
                (decoded, malformed),
                "{input:?}"
            );
        }
    }

    #[test]
    fn known_transfer_encodings() {
        for value in [
            &b"7bit"[..],
            b" 8BIT\r\n",
            b"binary (comment)",
            b"\r\n base64\r\n",
            b" Quoted-Printable",
        ] {
            assert!(is_known_transfer_encoding(value), "{value:?}");
        }
        for value in [
            &b"x-uuencode"[..],
            b"",
            b" base64;",
            b"7-bit",
            b"\"base64\"",
        ] {
            assert!(!is_known_transfer_encoding(value), "{value:?}");
        }
    }

    #[test]
    fn text_conversion() {
        assert!(matches!(
            to_text(Cow::Borrowed(b"caf\xc3\xa9"), charset(Some("utf-8"))),
            Cow::Borrowed("café")
        ));
        assert_eq!(
            to_text(Cow::Borrowed(b"caf\xe9"), charset(Some("iso-8859-1"))),
            "café"
        );
        assert_eq!(
            to_text(Cow::Owned(b"caf\xe9".to_vec()), charset(None)),
            "caf\u{fffd}"
        );
        assert_eq!(
            to_text(Cow::Owned(b"caf\xe9".to_vec()), charset(Some("x-unknown"))),
            "caf\u{fffd}"
        );
    }
}
