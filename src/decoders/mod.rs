/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Decoders the parser uses that are also useful on their own: charset
//! conversion, HTML to text and text to HTML, and body previews. Transfer
//! encodings (base64, quoted-printable, hex) are decoded by encodify; use
//! it directly for those, or [`crate::Encoding::decode`] to decode a body
//! as the parser does.

pub mod charsets;
pub mod html;
mod prefix;
pub mod preview;

use crate::Encoding;
use encodify::{base64, qp};
use std::borrow::Cow;

pub(crate) use prefix::{Limit, TextPrefix};

impl Encoding {
    pub(crate) fn parse(value: &[u8]) -> Option<Encoding> {
        let token = value
            .split(|&byte| byte == b'(')
            .next()
            .unwrap_or_default()
            .trim_ascii();
        hashify::map_ignore_case!(token, Encoding,
            "7bit" => Encoding::None,
            "8bit" => Encoding::None,
            "binary" => Encoding::None,
            "base64" => Encoding::Base64,
            "quoted-printable" => Encoding::QuotedPrintable,
        )
        .copied()
    }

    /// Decodes a body in this Content-Transfer-Encoding, as [`crate::MessagePart::decoded`] does.
    pub fn decode(self, bytes: &[u8]) -> Cow<'_, [u8]> {
        match self {
            Encoding::None => Cow::Borrowed(bytes),
            Encoding::QuotedPrintable => qp::BODY.decode(bytes).unwrap_or(Cow::Borrowed(bytes)),
            Encoding::Base64 => {
                let mut out = Vec::new();
                self.decode_append(bytes, &mut out);
                Cow::Owned(out)
            }
        }
    }

    /// Decodes a body like [`Encoding::decode`] and reports whether the encoding was malformed.
    pub fn decode_checked(self, bytes: &[u8]) -> (Cow<'_, [u8]>, bool) {
        match self {
            Encoding::None => (Cow::Borrowed(bytes), false),
            Encoding::QuotedPrintable => match qp::BODY.strict().decode(bytes) {
                Ok(decoded) => (decoded, false),
                Err(_) => (self.decode(bytes), true),
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

    /// Decodes a body like [`Encoding::decode`], appending the result to `out`.
    pub fn decode_append(self, bytes: &[u8], out: &mut Vec<u8>) {
        match self {
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

    /// Returns the length of a body after transfer decoding, as [`crate::MessagePart::decoded_len`] does.
    pub fn decoded_len(self, bytes: &[u8]) -> usize {
        match self {
            Encoding::None => bytes.len(),
            Encoding::Base64 => base64::MIME.decoded_len(bytes).unwrap_or_else(|_| {
                let mut out = Vec::new();
                base64_tolerant(bytes, &mut out);
                out.len()
            }),
            Encoding::QuotedPrintable => qp::BODY.decoded_len(bytes).unwrap_or(bytes.len()),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Charset, scan::tests::Rng};

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
            let borrowed = Encoding::QuotedPrintable.decode(&input);
            assert_eq!(borrowed.as_ref(), decoded.as_slice(), "{input:?}");
            if matches!(borrowed, Cow::Borrowed(_)) {
                assert_eq!(decoded, input);
            }
            assert_eq!(Encoding::QuotedPrintable.decoded_len(&input), decoded.len());
        }
        assert!(matches!(
            Encoding::QuotedPrintable.decode(b"plain text\r\nline two\nend"),
            Cow::Borrowed(_)
        ));
        assert!(matches!(
            Encoding::QuotedPrintable.decode(b"trailing \r\n"),
            Cow::Owned(_)
        ));
    }

    #[test]
    fn base64_best_effort() {
        assert_eq!(Encoding::Base64.decode(b"SGVs\r\nbG8=").as_ref(), b"Hello");
        assert_eq!(Encoding::Base64.decode(b"SGVs-bG8=").as_ref(), b"Hello");
        assert_eq!(Encoding::Base64.decode(b"VGVzdA").as_ref(), b"Test");
        assert_eq!(Encoding::Base64.decoded_len(b"SGVs-bG8="), 5);
        assert_eq!(Encoding::Base64.decoded_len(b"SGVsbG8="), 5);
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
                let (checked, malformed) = encoding.decode_checked(&input);
                assert_eq!(checked, encoding.decode(&input), "{input:?}");
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
            let (checked, problem) = encoding.decode_checked(input);
            assert_eq!(
                (checked.as_ref(), problem),
                (decoded, malformed),
                "{input:?}"
            );
        }
    }

    #[test]
    fn known_transfer_encodings() {
        for (value, encoding) in [
            (&b"7bit"[..], Encoding::None),
            (b" 8BIT\r\n", Encoding::None),
            (b"binary (comment)", Encoding::None),
            (b"\r\n base64\r\n", Encoding::Base64),
            (b" BASE64 (comment)\n", Encoding::Base64),
            (b" Quoted-Printable", Encoding::QuotedPrintable),
        ] {
            assert_eq!(Encoding::parse(value), Some(encoding), "{value:?}");
        }
        for value in [
            &b"x-uuencode"[..],
            b"",
            b" base64;",
            b"7-bit",
            b"\"base64\"",
        ] {
            assert_eq!(Encoding::parse(value), None, "{value:?}");
        }
    }

    #[test]
    fn text_conversion() {
        let label = |label: &str| Charset::from_label(label.as_bytes()).unwrap_or_default();
        assert!(matches!(
            label("utf-8").decode_cow(Cow::Borrowed(b"caf\xc3\xa9")),
            Cow::Borrowed("café")
        ));
        assert_eq!(
            label("iso-8859-1").decode_cow(Cow::Borrowed(b"caf\xe9")),
            "café"
        );
        assert_eq!(
            Charset::default().decode_cow(Cow::Owned(b"caf\xe9".to_vec())),
            "caf\u{fffd}"
        );
        assert_eq!(
            label("x-unknown").decode_cow(Cow::Owned(b"caf\xe9".to_vec())),
            "caf\u{fffd}"
        );
    }
}
