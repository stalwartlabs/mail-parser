/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Charset conversion to UTF-8.
//!
//! [`Charset::from_label`] maps a MIME or WHATWG label to a [`Charset`], and
//! [`Charset::decode`] converts bytes. The single-byte charsets, UTF-7 and
//! UTF-16 are built in; the multi-byte charsets (Shift_JIS, EUC-JP,
//! ISO-2022-JP, EUC-KR, Big5, GBK, GB18030) and a few legacy single-byte
//! ones need the `full_encoding` feature, which uses `encoding_rs`.
//!
//! ```
//! use mail_parser::Charset;
//!
//! let charset = Charset::from_label(b"ISO-8859-1").expect("known label");
//! assert_eq!(charset.decode(b"caf\xe9"), "caf\u{e9}");
//! assert_eq!(Charset::from_label(b"x-unknown"), None);
//! ```

mod ascii;
mod label;
mod multi_byte;
mod single_byte;
mod tables;
mod utf16;
mod utf7;
mod utf8;

use std::borrow::Cow;

use multi_byte::MultiByte;
use single_byte::Table;
use utf16::Endian;

const MAX_LABEL_LEN: usize = 45;
const UTF8_LABEL: &[u8] = b"utf-8";

/// A charset decoder function, as returned by [`charset_decoder`].
pub type DecoderFnc = fn(&[u8]) -> String;

/// A charset that text can be converted from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Charset {
    /// UTF-8, also used for text without a charset or with an unknown one.
    #[default]
    Utf8,
    /// UTF-7 (RFC 2152).
    Utf7,
    /// UTF-16 with a byte order mark; little-endian without one.
    Utf16,
    /// UTF-16, little-endian.
    Utf16Le,
    /// UTF-16, big-endian.
    Utf16Be,
    /// ISO-8859-2 (Latin-2, Central European).
    Iso8859_2,
    /// ISO-8859-3 (Latin-3, South European).
    Iso8859_3,
    /// ISO-8859-4 (Latin-4, North European).
    Iso8859_4,
    /// ISO-8859-5 (Cyrillic).
    Iso8859_5,
    /// ISO-8859-6 (Arabic).
    Iso8859_6,
    /// ISO-8859-7 (Greek).
    Iso8859_7,
    /// ISO-8859-8 (Hebrew).
    Iso8859_8,
    /// ISO-8859-9 (Latin-5, Turkish), strictly as the standard defines it.
    /// No label resolves here: `iso-8859-9` and its aliases resolve to
    /// [`Charset::Windows1254`], the superset senders actually send.
    Iso8859_9,
    /// ISO-8859-10 (Latin-6, Nordic).
    Iso8859_10,
    /// ISO-8859-13 (Latin-7, Baltic).
    Iso8859_13,
    /// ISO-8859-14 (Latin-8, Celtic).
    Iso8859_14,
    /// ISO-8859-15 (Latin-9).
    Iso8859_15,
    /// ISO-8859-16 (Latin-10, South-Eastern European).
    Iso8859_16,
    /// windows-1250 (Central European).
    Windows1250,
    /// windows-1251 (Cyrillic).
    Windows1251,
    /// windows-1252 (Western European), also used for the ISO-8859-1 and
    /// US-ASCII labels, as browsers do.
    Windows1252,
    /// windows-1253 (Greek).
    Windows1253,
    /// windows-1254 (Turkish).
    Windows1254,
    /// windows-1255 (Hebrew).
    Windows1255,
    /// windows-1256 (Arabic).
    Windows1256,
    /// windows-1257 (Baltic).
    Windows1257,
    /// windows-1258 (Vietnamese).
    Windows1258,
    /// KOI8-R (Russian).
    Koi8R,
    /// KOI8-U (Ukrainian).
    Koi8U,
    /// Mac OS Roman.
    Macintosh,
    /// IBM code page 850 (DOS Latin-1).
    Ibm850,
    /// TIS-620 (Thai), strictly as the standard defines it. No label resolves
    /// here: `tis-620` and `iso-8859-11` resolve to [`Charset::Windows874`],
    /// the superset senders actually send.
    Tis620,
    /// Shift_JIS (Japanese); needs the `full_encoding` feature.
    ShiftJis,
    /// Big5 (Traditional Chinese); needs the `full_encoding` feature.
    Big5,
    /// EUC-JP (Japanese); needs the `full_encoding` feature.
    EucJp,
    /// EUC-KR (Korean); needs the `full_encoding` feature.
    EucKr,
    /// GB18030 (Simplified Chinese); needs the `full_encoding` feature.
    Gb18030,
    /// GBK (Simplified Chinese); needs the `full_encoding` feature.
    Gbk,
    /// ISO-2022-JP (Japanese); needs the `full_encoding` feature.
    Iso2022Jp,
    /// windows-874 (Thai). Also the target of the `tis-620` and `iso-8859-11`
    /// labels, since senders use them for code page 874 bytes.
    Windows874,
    /// IBM code page 866 (DOS Cyrillic); needs the `full_encoding` feature.
    Ibm866,
    /// x-mac-cyrillic; needs the `full_encoding` feature.
    XMacCyrillic,
    /// x-user-defined; needs the `full_encoding` feature.
    XUserDefined,
    /// The WHATWG replacement encoding, for labels of charsets that are not
    /// decoded on purpose (ISO-2022-KR, HZ-GB-2312, ...): the text becomes one
    /// U+FFFD. Needs the `full_encoding` feature.
    Replacement,
}

enum Decoder {
    Utf8,
    Utf7,
    Utf16Bom,
    Utf16(Endian),
    SingleByte(&'static Table),
    MultiByte(MultiByte),
}

impl Charset {
    /// The charset for a MIME or WHATWG label, ASCII case-insensitive;
    /// `None` for an unknown label.
    pub fn from_label(label: &[u8]) -> Option<Charset> {
        label::lookup(label.get(..MAX_LABEL_LEN).unwrap_or(label))
    }

    /// Converts `bytes` to text; byte sequences that are not valid in the
    /// charset become U+FFFD. Borrowed when the bytes are already the text
    /// (valid UTF-8, or ASCII in a single-byte charset).
    pub fn decode(self, bytes: &[u8]) -> Cow<'_, str> {
        match self.decoder() {
            Decoder::Utf8 => utf8::decode(bytes),
            Decoder::Utf7 => utf7::decode(bytes),
            Decoder::Utf16Bom => Cow::Owned(utf16::decode_bom(bytes)),
            Decoder::Utf16(endian) => Cow::Owned(endian.decode(bytes)),
            Decoder::SingleByte(table) => table.decode(bytes),
            Decoder::MultiByte(encoding) => encoding.decode(bytes),
        }
    }

    /// Converts `bytes` to text like [`Charset::decode`], reusing the vector
    /// when the bytes are already the text.
    pub fn decode_owned(self, bytes: Vec<u8>) -> String {
        let decoded = match self.decoder() {
            Decoder::Utf8 => return utf8::decode_owned(bytes),
            Decoder::Utf7 => utf7::decode(&bytes),
            Decoder::Utf16Bom => return utf16::decode_bom(&bytes),
            Decoder::Utf16(endian) => return endian.decode(&bytes),
            Decoder::SingleByte(table) => table.decode(&bytes),
            Decoder::MultiByte(encoding) => encoding.decode(&bytes),
        };
        let text = match decoded {
            Cow::Owned(text) => return text,
            Cow::Borrowed(text) => text,
        };
        if text.len() == bytes.len() {
            utf8::decode_owned(bytes)
        } else {
            text.to_owned()
        }
    }

    pub(crate) fn decode_prefix(self, bytes: &[u8]) -> Cow<'_, str> {
        match self.decoder() {
            Decoder::Utf8 => utf8::decode(utf8::complete_prefix(bytes)),
            Decoder::Utf7 => utf7::decode(utf7::complete_prefix(bytes)),
            Decoder::Utf16Bom => Cow::Owned(utf16::decode_bom(utf16::complete_prefix_bom(bytes))),
            Decoder::Utf16(endian) => Cow::Owned(endian.decode(endian.complete_prefix(bytes))),
            Decoder::SingleByte(table) => table.decode(bytes),
            Decoder::MultiByte(encoding) => encoding.decode_prefix(bytes),
        }
    }

    /// Converts `bytes` like [`Charset::decode`], appending the text to
    /// `out`.
    pub fn decode_append(self, bytes: &[u8], out: &mut String) {
        match self.decoder() {
            Decoder::Utf8 => utf8::decode_append(bytes, out),
            Decoder::Utf7 => utf7::decode_append(bytes, out),
            Decoder::Utf16Bom => utf16::decode_bom_append(bytes, out),
            Decoder::Utf16(endian) => endian.decode_append(bytes, out),
            Decoder::SingleByte(table) => table.decode_append(bytes, out),
            Decoder::MultiByte(encoding) => encoding.decode_append(bytes, out),
        }
    }

    pub(crate) fn is_supported(self) -> bool {
        cfg!(feature = "full_encoding") || !matches!(self.decoder(), Decoder::MultiByte(_))
    }

    /// Converts `bytes` to text like [`Charset::decode`] and reports whether any sequence was not valid in the charset.
    pub fn decode_checked(self, bytes: &[u8]) -> (Cow<'_, str>, bool) {
        match self.decoder() {
            Decoder::Utf8 => utf8::decode_checked(bytes),
            Decoder::Utf7 => utf7::decode_checked(bytes),
            Decoder::Utf16Bom => {
                let (text, malformed) = utf16::decode_bom_checked(bytes);
                (Cow::Owned(text), malformed)
            }
            Decoder::Utf16(endian) => {
                let (text, malformed) = endian.decode_checked(bytes);
                (Cow::Owned(text), malformed)
            }
            Decoder::SingleByte(table) => {
                let text = table.decode(bytes);
                let malformed = text.contains(char::REPLACEMENT_CHARACTER);
                (text, malformed)
            }
            Decoder::MultiByte(encoding) => encoding.decode_checked(bytes),
        }
    }

    pub(crate) fn decode_owned_checked(self, bytes: Vec<u8>) -> (String, bool) {
        if let Decoder::Utf8 = self.decoder() {
            return utf8::decode_owned_checked(bytes);
        }
        let (decoded, malformed) = self.decode_checked(&bytes);
        let text = match decoded {
            Cow::Owned(text) => return (text, malformed),
            Cow::Borrowed(text) => text,
        };
        if text.len() == bytes.len() {
            (utf8::decode_owned(bytes), malformed)
        } else {
            (text.to_owned(), malformed)
        }
    }

    pub(crate) fn decode_cow(self, bytes: Cow<'_, [u8]>) -> Cow<'_, str> {
        match bytes {
            Cow::Borrowed(bytes) => self.decode(bytes),
            Cow::Owned(bytes) => Cow::Owned(self.decode_owned(bytes)),
        }
    }

    pub(crate) fn decode_cow_checked(self, bytes: Cow<'_, [u8]>) -> (Cow<'_, str>, bool) {
        match bytes {
            Cow::Borrowed(bytes) => self.decode_checked(bytes),
            Cow::Owned(bytes) => {
                let (text, malformed) = self.decode_owned_checked(bytes);
                (Cow::Owned(text), malformed)
            }
        }
    }

    fn decoder(self) -> Decoder {
        match self {
            Charset::Utf8 => Decoder::Utf8,
            Charset::Utf7 => Decoder::Utf7,
            Charset::Utf16 => Decoder::Utf16Bom,
            Charset::Utf16Le => Decoder::Utf16(Endian::Little),
            Charset::Utf16Be => Decoder::Utf16(Endian::Big),
            Charset::Iso8859_2 => Decoder::SingleByte(&tables::ISO_8859_2),
            Charset::Iso8859_3 => Decoder::SingleByte(&tables::ISO_8859_3),
            Charset::Iso8859_4 => Decoder::SingleByte(&tables::ISO_8859_4),
            Charset::Iso8859_5 => Decoder::SingleByte(&tables::ISO_8859_5),
            Charset::Iso8859_6 => Decoder::SingleByte(&tables::ISO_8859_6),
            Charset::Iso8859_7 => Decoder::SingleByte(&tables::ISO_8859_7),
            Charset::Iso8859_8 => Decoder::SingleByte(&tables::ISO_8859_8),
            Charset::Iso8859_9 => Decoder::SingleByte(&tables::ISO_8859_9),
            Charset::Iso8859_10 => Decoder::SingleByte(&tables::ISO_8859_10),
            Charset::Iso8859_13 => Decoder::SingleByte(&tables::ISO_8859_13),
            Charset::Iso8859_14 => Decoder::SingleByte(&tables::ISO_8859_14),
            Charset::Iso8859_15 => Decoder::SingleByte(&tables::ISO_8859_15),
            Charset::Iso8859_16 => Decoder::SingleByte(&tables::ISO_8859_16),
            Charset::Windows1250 => Decoder::SingleByte(&tables::CP1250),
            Charset::Windows1251 => Decoder::SingleByte(&tables::CP1251),
            Charset::Windows1252 => Decoder::SingleByte(&tables::CP1252),
            Charset::Windows1253 => Decoder::SingleByte(&tables::CP1253),
            Charset::Windows1254 => Decoder::SingleByte(&tables::CP1254),
            Charset::Windows1255 => Decoder::SingleByte(&tables::CP1255),
            Charset::Windows1256 => Decoder::SingleByte(&tables::CP1256),
            Charset::Windows1257 => Decoder::SingleByte(&tables::CP1257),
            Charset::Windows1258 => Decoder::SingleByte(&tables::CP1258),
            Charset::Koi8R => Decoder::SingleByte(&tables::KOI8_R),
            Charset::Koi8U => Decoder::SingleByte(&tables::KOI8_U),
            Charset::Macintosh => Decoder::SingleByte(&tables::MACINTOSH),
            Charset::Ibm850 => Decoder::SingleByte(&tables::IBM850),
            Charset::Tis620 => Decoder::SingleByte(&tables::TIS_620),
            Charset::ShiftJis => Decoder::MultiByte(MultiByte::ShiftJis),
            Charset::Big5 => Decoder::MultiByte(MultiByte::Big5),
            Charset::EucJp => Decoder::MultiByte(MultiByte::EucJp),
            Charset::EucKr => Decoder::MultiByte(MultiByte::EucKr),
            Charset::Gb18030 => Decoder::MultiByte(MultiByte::Gb18030),
            Charset::Gbk => Decoder::MultiByte(MultiByte::Gbk),
            Charset::Iso2022Jp => Decoder::MultiByte(MultiByte::Iso2022Jp),
            Charset::Windows874 => Decoder::SingleByte(&tables::WINDOWS_874),
            Charset::Ibm866 => Decoder::MultiByte(MultiByte::Ibm866),
            Charset::XMacCyrillic => Decoder::MultiByte(MultiByte::XMacCyrillic),
            Charset::XUserDefined => Decoder::MultiByte(MultiByte::XUserDefined),
            Charset::Replacement => Decoder::MultiByte(MultiByte::Replacement),
        }
    }
}

/// Converts `bytes` from the charset with this label; UTF-8 when the label
/// is unknown.
pub fn decode<'x>(charset: &[u8], bytes: &'x [u8]) -> Cow<'x, str> {
    Charset::from_label(charset)
        .unwrap_or_default()
        .decode(bytes)
}

/// Converts `bytes` from the charset with this label (UTF-8 when the label
/// is unknown), appending the text to `out`. The signature fits the charset
/// callback of the `encodify::rfc2047` decoders.
pub fn decode_append(charset: &[u8], bytes: &[u8], out: &mut String) {
    let charset = if charset.eq_ignore_ascii_case(UTF8_LABEL) {
        Charset::Utf8
    } else {
        Charset::from_label(charset).unwrap_or_default()
    };
    charset.decode_append(bytes, out);
}

/// The 0.11 decoder lookup: a function converting text from the charset
/// with this label, or `None` for UTF-8 and unknown labels.
/// [`Charset::from_label`] and [`Charset::decode`] do the same without a
/// function pointer and borrow when they can.
pub fn charset_decoder(charset: &[u8]) -> Option<DecoderFnc> {
    macro_rules! decoder_fnc {
        ($charset:expr, $($variant:ident),+) => {
            match $charset {
                Charset::Utf8 => None,
                $(Charset::$variant => Some(|bytes| Charset::$variant.decode(bytes).into_owned()),)+
            }
        };
    }

    decoder_fnc!(
        Charset::from_label(charset)?,
        Utf7,
        Utf16,
        Utf16Le,
        Utf16Be,
        Iso8859_2,
        Iso8859_3,
        Iso8859_4,
        Iso8859_5,
        Iso8859_6,
        Iso8859_7,
        Iso8859_8,
        Iso8859_9,
        Iso8859_10,
        Iso8859_13,
        Iso8859_14,
        Iso8859_15,
        Iso8859_16,
        Windows1250,
        Windows1251,
        Windows1252,
        Windows1253,
        Windows1254,
        Windows1255,
        Windows1256,
        Windows1257,
        Windows1258,
        Koi8R,
        Koi8U,
        Macintosh,
        Ibm850,
        Tis620,
        ShiftJis,
        Big5,
        EucJp,
        EucKr,
        Gb18030,
        Gbk,
        Iso2022Jp,
        Windows874,
        Ibm866,
        XMacCyrillic,
        XUserDefined,
        Replacement
    )
}

#[cfg(test)]
mod tests {
    use super::{Charset, charset_decoder, decode_append, utf8};
    use crate::scan::tests::Rng;
    use std::borrow::Cow;

    const ALL: [Charset; 44] = [
        Charset::Utf8,
        Charset::Utf7,
        Charset::Utf16,
        Charset::Utf16Le,
        Charset::Utf16Be,
        Charset::Iso8859_2,
        Charset::Iso8859_3,
        Charset::Iso8859_4,
        Charset::Iso8859_5,
        Charset::Iso8859_6,
        Charset::Iso8859_7,
        Charset::Iso8859_8,
        Charset::Iso8859_9,
        Charset::Iso8859_10,
        Charset::Iso8859_13,
        Charset::Iso8859_14,
        Charset::Iso8859_15,
        Charset::Iso8859_16,
        Charset::Windows1250,
        Charset::Windows1251,
        Charset::Windows1252,
        Charset::Windows1253,
        Charset::Windows1254,
        Charset::Windows1255,
        Charset::Windows1256,
        Charset::Windows1257,
        Charset::Windows1258,
        Charset::Koi8R,
        Charset::Koi8U,
        Charset::Macintosh,
        Charset::Ibm850,
        Charset::Tis620,
        Charset::ShiftJis,
        Charset::Big5,
        Charset::EucJp,
        Charset::EucKr,
        Charset::Gb18030,
        Charset::Gbk,
        Charset::Iso2022Jp,
        Charset::Windows874,
        Charset::Ibm866,
        Charset::XMacCyrillic,
        Charset::XUserDefined,
        Charset::Replacement,
    ];

    const CHECKED_SAMPLES: usize = if cfg!(miri) { 30 } else { 3_000 };

    #[test]
    fn checked_decoding_returns_the_same_text() {
        let alphabet =
            b"aZ09 +-/=\r\n\x00\x1b$B(\x80\x81\xa4\xa5\xbf\xc3\xa9\xd8\xdc\xe9\xef\xfe\xff";
        let mut rng = Rng(0x0dec_0de0_c4ec_4ed0);
        for charset in ALL {
            for _ in 0..CHECKED_SAMPLES {
                let len = rng.below(24);
                let bytes: Vec<u8> = (0..len)
                    .map(|_| {
                        alphabet
                            .get(rng.below(alphabet.len()))
                            .copied()
                            .unwrap_or(b'a')
                    })
                    .collect();
                let (checked, malformed) = charset.decode_checked(&bytes);
                assert_eq!(checked, charset.decode(&bytes), "{charset:?} {bytes:?}");
                let (owned, owned_malformed) = charset.decode_owned_checked(bytes.clone());
                assert_eq!(
                    owned,
                    charset.decode_owned(bytes.clone()),
                    "{charset:?} {bytes:?}"
                );
                assert_eq!(owned_malformed, malformed, "{charset:?} {bytes:?}");
                if !malformed
                    && matches!(charset, Charset::Utf8 | Charset::Utf16Le | Charset::Utf16Be)
                {
                    assert!(
                        !checked.contains(char::REPLACEMENT_CHARACTER),
                        "{charset:?} {bytes:?}"
                    );
                }
                if !charset.is_supported() {
                    assert_eq!(malformed, matches!(checked, Cow::Owned(_)));
                }
            }
        }
    }

    #[test]
    fn owned_decoding_matches_borrowed_decoding() {
        let mut rng = Rng(0x0dec_0de0_c4ec_4ed1);
        for charset in ALL {
            for len in [
                utf8::SIMD_MIN - 1,
                utf8::SIMD_MIN,
                utf8::SIMD_MIN + 1,
                3 * utf8::SIMD_MIN,
            ] {
                for rare in [None, Some(b'\xe9'), Some(b'\xff'), Some(b'\xc3')] {
                    let mut bytes: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
                    if let (Some(byte), Some(slot)) = (rare, bytes.get_mut(rng.below(len))) {
                        *slot = byte;
                    }
                    let expected = charset.decode(&bytes).into_owned();
                    let (_, malformed) = charset.decode_checked(&bytes);
                    assert_eq!(
                        charset.decode_owned(bytes.clone()),
                        expected,
                        "{charset:?} {len}"
                    );
                    assert_eq!(
                        charset.decode_owned_checked(bytes),
                        (expected, malformed),
                        "{charset:?} {len}"
                    );
                }
            }
        }
    }

    #[test]
    fn malformed_charset_sequences() {
        let cases: [(Charset, &[u8], bool); 27] = [
            (Charset::Utf8, b"caf\xc3\xa9", false),
            (Charset::Utf8, b"caf\xe9", true),
            (Charset::Utf8, b"\xef\xbf\xbd", false),
            (Charset::Utf8, b"", false),
            (Charset::Iso8859_3, b"caf\xe9", false),
            (Charset::Iso8859_3, b"\xa5", true),
            (Charset::Windows1252, b"\x80\xff", false),
            (Charset::Windows1252, b"\x81", true),
            (Charset::Utf16Le, b"a\x00b\x00", false),
            (Charset::Utf16Le, b"a\x00b", true),
            (Charset::Utf16Le, b"\x00\xd8a\x00", true),
            (Charset::Utf16Be, b"\xff\xfd", false),
            (Charset::Utf16, b"\xfe\xff\x00a", false),
            (Charset::Utf16, b"\xfe\xff\x00a\x00", true),
            (Charset::Utf7, b"Hi Mom -+Jjo--!", false),
            (Charset::Utf7, b"+AKM-1", false),
            (Charset::Utf7, b"Hi +AKM", true),
            (Charset::Utf7, b"caf\xc3\xa9", true),
            (Charset::Utf7, b"+2D0-", true),
            (Charset::Utf7, b"+A-", true),
            (Charset::Utf7, b"a+", true),
            (Charset::Utf7, b"1 +- 1", false),
            (Charset::Utf7, b"x +AKM y", false),
            (Charset::Utf7, b"+AKM\xe9", true),
            (Charset::Utf7, b"+//0-", false),
            (Charset::ShiftJis, b"plain", false),
            (Charset::Replacement, b"", false),
        ];
        for (charset, bytes, expected) in cases {
            let malformed = charset.decode_checked(bytes).1;
            assert_eq!(malformed, expected, "{charset:?} {bytes:?}");
            assert_eq!(
                charset.decode_owned_checked(bytes.to_vec()).1,
                malformed,
                "{charset:?} {bytes:?}"
            );
        }
        #[cfg(feature = "full_encoding")]
        {
            assert!(!Charset::ShiftJis.decode_checked(b"\x83n\x83\x8D").1);
            assert!(Charset::ShiftJis.decode_checked(b"\x83").1);
            assert!(Charset::Replacement.decode_checked(b"abc").1);
            assert!(Charset::ShiftJis.is_supported());
        }
        #[cfg(not(feature = "full_encoding"))]
        assert!(!Charset::ShiftJis.is_supported());
        assert!(Charset::Iso8859_2.is_supported() && Charset::Utf16.is_supported());
    }

    #[test]
    fn utf8_label_shortcut_matches_lookup() {
        for label in ["utf-8", "UTF-8", "Utf-8", "utf8", "iso-8859-1", "x", ""] {
            for bytes in [&b"caf\xc3\xa9"[..], b"caf\xe9 \xff", b"\xc3", b""] {
                let mut expected = String::new();
                Charset::from_label(label.as_bytes())
                    .unwrap_or_default()
                    .decode_append(bytes, &mut expected);
                let mut actual = String::new();
                decode_append(label.as_bytes(), bytes, &mut actual);
                assert_eq!(actual, expected, "{label:?} {bytes:?}");
            }
        }
    }

    #[test]
    fn decode_charset() {
        let inputs = [
            ("iso-8859-1", b"\xe1\xe9\xed\xf3\xfa".to_vec(), "áéíóú"),
            ("iso-8859-1", b"\x805.4bn".to_vec(), "€5.4bn"),
            ("latin1", b"\x805.4bn".to_vec(), "€5.4bn"),
            ("iso88591", b"\x805.4bn".to_vec(), "€5.4bn"),
            ("us-ascii", b"\x805.4bn".to_vec(), "€5.4bn"),
            ("iso8859-5", b"\xbf\xe0\xd8\xd2\xd5\xe2, \xdc\xd8\xe0".to_vec(), "Привет, мир"),
            ("cp1252", b"\xa1El \xf1and\xfa comi\xf3 \xf1oquis!".to_vec(), "¡El ñandú comió ñoquis!"),
            ("iso-8859-5", b"\xbf\xe0\xd8\xd2\xd5\xe2, \xdc\xd8\xe0".to_vec(), "Привет, мир"),
            ("iso-8859-6", b"\xe5\xd1\xcd\xc8\xc7 \xc8\xc7\xe4\xd9\xc7\xe4\xe5".to_vec(),"مرحبا بالعالم"),
            ("iso-8859-7", b"\xc3\xe5\xe9\xdc \xf3\xef\xf5 \xca\xfc\xf3\xec\xe5".to_vec(),"Γειά σου Κόσμε"),
            ("iso-8859-8", b"\xf9\xec\xe5\xed \xf2\xe5\xec\xed".to_vec(),"שלום עולם"),
            ("iso-8859-11", b"\xc3\xcb\xd1\xca\xca\xd3\xcb\xc3\xd1\xba\xcd\xd1\xa1\xa2\xc3\xd0\xe4\xb7\xc2\xb7\xd5\xe8\xe3\xaa\xe9\xa1\xd1\xba\xa4\xcd\xc1\xbe\xd4\xc7\xe0\xb5\xcd\xc3\xec".to_vec(),"รหัสสำหรับอักขระไทยที่ใช้กับคอมพิวเตอร์"),
            ("windows-1250", b"Zelo rada grem v sla\x9a\xe8i\xe8arno".to_vec(),"Zelo rada grem v slaščičarno"),
            ("windows-1251", b"\xcf\xf0\xe8\xe2\xe5\xf2, \xec\xe8\xf0".to_vec(),"Привет, мир"),
            ("windows-1252", b"\xa1El \xf1and\xfa comi\xf3 \xf1oquis!".to_vec(),"¡El ñandú comió ñoquis!"),
            ("windows-1253", b"\xca\xf9\xe4\xe9\xea\xef\xdf \xd3\xf9\xea\xf1\xdc\xf4\xe7\xf2 \xf3\xf4\xef Rust".to_vec(),"Κωδικοί Σωκράτης στο Rust"),
            ("windows-1254", b"Kebab\xfdm\xfd baharatl\xfd yapma".to_vec(),"Kebabımı baharatlı yapma"),
            ("windows-1255", b"\xf9\xec\xe5\xed \xf2\xe5\xec\xed".to_vec(),"שלום עולם"),
            ("windows-1256", b"\xe3\xd1\xcd\xc8\xc7 \xc8\xc7\xe1\xda\xc7\xe1\xe3".to_vec(),"مرحبا بالعالم"),
            ("windows-1257", b"Mu h\xf5ljuk on angerjaid t\xe4is".to_vec(),"Mu hõljuk on angerjaid täis"),
            ("windows-1258", b"Xin ch\xe0o".to_vec(),"Xin chào"),
            ("macintosh", b"\x87\x8e\x92\x97\x9c".to_vec(),"áéíóú"),
            ("ibm850", b"\x9b\x9c\x9d\x9e".to_vec(),"ø£Ø×"),
            ("koi8-r", b"\xf0\xd2\xc9\xd7\xc5\xd4, \xcd\xc9\xd2".to_vec(),"Привет, мир"),
            ("koi8-u", b"\xf0\xd2\xc9\xd7\xa6\xd4 \xf3\xd7\xa6\xd4".to_vec(),"Привіт Світ"),
            // Positions that ISO 8859-7:2003 added over the 1987 edition.
            ("iso-8859-7", b"\xa4\xa5\xaa".to_vec(),"€₯ͺ"),
            // 0xa5 and 0xab were transposed.
            ("iso-8859-16", b"\xa5\xab".to_vec(),"„«"),
            // 0xdb became the euro sign in Mac OS 8.5; 0xf0 is the Apple logo.
            ("macintosh", b"\xc6\xcd\xdb\xf0\xf6\xf7".to_vec(),"∆Õ€\u{f8ff}ˆ˜"),
            // Hebrew point holam haser for vav, a later windows-1255 addition.
            ("windows-1255", b"\xca".to_vec(),"\u{5ba}"),
            // The iso-8859-9 and tis-620 labels resolve to the Windows
            // supersets, so bytes in 0x80..=0xa0 decode instead of being lost.
            ("iso-8859-9", b"\x93Merhaba\x94 \x96 d\xfcnya".to_vec(),"“Merhaba” – dünya"),
            ("iso-8859-9", b"\x805.4bn".to_vec(),"€5.4bn"),
            ("latin5", b"\x805.4bn".to_vec(),"€5.4bn"),
            ("tis-620", b"\x93\xca\xc7\xd1\xca\xb4\xd5\x94".to_vec(),"“สวัสดี”"),
            ("iso-8859-11", b"\x805.4bn".to_vec(),"€5.4bn"),
            // Thai text decodes as before; only the 0x80..=0xa0 band changed.
            ("tis-620", b"\xc3\xcb\xd1\xca".to_vec(),"รหัส"),
            ("utf-7", b"+ZYeB9FH6ckh5Pg-, 1980.".to_vec(),"文致出版社, 1980."),
            ("utf-16le", b"\xcf0\xed0\xfc0\xfb0\xef0\xfc0\xeb0\xc90".to_vec(),"ハロー・ワールド"),
            ("utf-16be", b"0\xcf0\xed0\xfc0\xfb0\xef0\xfc0\xeb0\xc9".to_vec(),"ハロー・ワールド"),
            ("utf-16", b"\xff\xfe\xe1\x00\xe9\x00\xed\x00\xf3\x00\xfa\x00".to_vec(),"áéíóú"),
            ("utf-16", b"\xfe\xff\x00\xe1\x00\xe9\x00\xed\x00\xf3\x00\xfa".to_vec(),"áéíóú"),

            #[cfg(feature = "full_encoding")]
            ("shift_jis", b"\x83n\x83\x8D\x81[\x81E\x83\x8F\x81[\x83\x8B\x83h".to_vec(),"ハロー・ワールド"),
            #[cfg(feature = "full_encoding")]
            ("big5", b"\xa7A\xa6n\xa1A\xa5@\xac\xc9".to_vec(),"你好，世界"),
            #[cfg(feature = "full_encoding")]
            ("euc-jp", b"\xa5\xcf\xa5\xed\xa1\xbc\xa1\xa6\xa5\xef\xa1\xbc\xa5\xeb\xa5\xc9".to_vec(),"ハロー・ワールド"),
            #[cfg(feature = "full_encoding")]
            ("euc-kr", b"\xbe\xc8\xb3\xe7\xc7\xcf\xbc\xbc\xbf\xe4 \xbc\xbc\xb0\xe8".to_vec(),"안녕하세요 세계"),
            #[cfg(feature = "full_encoding")]
            ("cp949", b"\xbe\xc8\xb3\xe7\xc7\xcf\xbc\xbc\xbf\xe4 \xbc\xbc\xb0\xe8".to_vec(),"안녕하세요 세계"),
            #[cfg(feature = "full_encoding")]
            ("uhc", b"\xbe\xc8\xb3\xe7\xc7\xcf\xbc\xbc\xbf\xe4 \xbc\xbc\xb0\xe8".to_vec(),"안녕하세요 세계"),
            #[cfg(feature = "full_encoding")]
            ("x-windows-949", b"\xbe\xc8\xb3\xe7\xc7\xcf\xbc\xbc\xbf\xe4 \xbc\xbc\xb0\xe8".to_vec(),"안녕하세요 세계"),
            #[cfg(feature = "full_encoding")]
            ("windows-949", b"\xbe\xc8\xb3\xe7\xc7\xcf\xbc\xbc\xbf\xe4 \xbc\xbc\xb0\xe8".to_vec(),"안녕하세요 세계"),
            #[cfg(feature = "full_encoding")]
            ("iso-2022-jp", b"\x1b$B%O%m!<!&%o!<%k%I\x1b(B".to_vec(),"ハロー・ワールド"),
            #[cfg(feature = "full_encoding")]
            ("gbk", b"\xc4\xe3\xba\xc3\xa3\xac\xca\xc0\xbd\xe7".to_vec(),"你好，世界"),
            #[cfg(feature = "full_encoding")]
            ("gb18030", b"\xc4\xe3\xba\xc3\xa3\xac\xca\xc0\xbd\xe7".to_vec(),"你好，世界"),
            #[cfg(feature = "full_encoding")]
            ("x-mac-cyrillic", b"\x8f\xf0\xe8\xe2\xe5\xf2".to_vec(),"Привет"),
            #[cfg(feature = "full_encoding")]
            ("x-user-defined", b"\x80\xff".to_vec(),"\u{f780}\u{f7ff}"),
            #[cfg(feature = "full_encoding")]
            ("iso-2022-kr", b"\x1b$)Cabcd".to_vec(),"\u{fffd}"),
            #[cfg(feature = "full_encoding")]
            ("hz-gb-2312", b"~{...~}".to_vec(),"\u{fffd}"),
            ];

        for (label, bytes, expected) in inputs {
            let decoder = charset_decoder(label.as_bytes()).expect(label);
            assert_eq!(decoder(&bytes), expected, "{label}");
            let charset = Charset::from_label(label.as_bytes()).expect(label);
            assert_eq!(charset.decode(&bytes), expected, "{label}");
            let mut out = String::from(">");
            charset.decode_append(&bytes, &mut out);
            assert_eq!(out.strip_prefix('>'), Some(expected), "{label}");
        }
    }

    #[test]
    fn decoder_charset() {
        for input in ["gbk", "extended_unix_code_packed_format_for_japanese"] {
            if !input.is_empty() {
                assert!(
                    charset_decoder(input.as_bytes()).is_some(),
                    "Failed for {input}",
                );
            }
        }
    }

    #[test]
    fn decoder_charset_encoding_rs_labels() {
        let supported = [
            "l9",
            "koi",
            "koi8",
            "sjis",
            "ucs-2",
            "ms932",
            "ascii",
            "x-gbk",
            "cp1250",
            "cp1251",
            "cp1252",
            "cp1253",
            "cp1254",
            "cp1255",
            "cp1256",
            "cp1257",
            "cp1258",
            "visual",
            "korean",
            "x-sjis",
            "ksc5601",
            "gb_2312",
            "dos-874",
            "cn-big5",
            "unicode",
            "chinese",
            "logical",
            "koi8-ru",
            "x-cp1250",
            "ksc_5601",
            "x-cp1251",
            "iso88591",
            "csgb2312",
            "x-cp1252",
            "iso88592",
            "x-cp1253",
            "iso88593",
            "x-cp1254",
            "iso88594",
            "x-cp1255",
            "iso88595",
            "x-x-big5",
            "x-cp1256",
            "iso88596",
            "x-cp1257",
            "iso88597",
            "x-cp1258",
            "iso88598",
            "iso88599",
            "us-ascii",
            "x-euc-jp",
            "iso885910",
            "iso8859-1",
            "iso885911",
            "iso8859-2",
            "iso8859-3",
            "iso885913",
            "iso8859-4",
            "iso885914",
            "iso8859-5",
            "iso885915",
            "iso8859-6",
            "iso8859-7",
            "iso8859-8",
            "iso-ir-58",
            "iso8859-9",
            "csunicode",
            "iso8859-10",
            "gb_2312-80",
            "iso8859-11",
            "iso8859-13",
            "iso8859-14",
            "iso8859-15",
            "iso-ir-149",
            "big5-hkscs",
            "windows-949",
            "cp949",
            "uhc",
            "x-windows-949",
            "csisolatin9",
            "csiso88596e",
            "csiso88598e",
            "unicodefffe",
            "unicodefeff",
            "csiso88596i",
            "csiso88598i",
            "windows-31j",
            "x-mac-roman",
            "sun_eu_greek",
            "csksc56011987",
            "ansi_x3.4-1968",
            "csiso58gb231280",
            "iso-10646-ucs-2",
            "iso-8859-6-e",
            "iso-8859-8-e",
            "iso-8859-6-i",
            "replacement",
            "iso-2022-kr",
            "csiso2022kr",
            "iso-2022-cn",
            "iso-2022-cn-ext",
            "hz-gb-2312",
            "x-user-defined",
            "x-mac-cyrillic",
            "x-mac-ukrainian",
        ];
        for input in supported {
            assert!(
                charset_decoder(input.as_bytes()).is_some(),
                "Expected a decoder for {input}",
            );
        }

        let unsupported = [
            "utf8",
            "utf-8",
            "unicode11utf8",
            "unicode20utf8",
            "x-unicode20utf8",
            "unicode-1-1-utf-8",
        ];
        for input in unsupported {
            assert!(
                charset_decoder(input.as_bytes()).is_none(),
                "Did not expect a decoder for {input}",
            );
            assert_eq!(
                Charset::from_label(input.as_bytes()),
                Some(Charset::Utf8),
                "{input}"
            );
        }
        for input in ["", "unknown", "utf-9", "x-unknown-8bit"] {
            assert_eq!(Charset::from_label(input.as_bytes()), None, "{input}");
        }
    }

    #[test]
    fn decode_utf7() {
        let inputs = [
            ("Hello, World+ACE-", "Hello, World!"),
            ("Hi Mom -+Jjo--!", "Hi Mom -☺-!"),
            ("+ZeVnLIqe-", "日本語"),
            ("Item 3 is +AKM-1.", "Item 3 is £1."),
            ("Plus minus +- -+ +--", "Plus minus + -+ +-"),
            (
                "+APw-ber ihre mi+AN8-liche Lage+ADs- +ACI-wir",
                "über ihre mißliche Lage; \"wir",
            ),
            (
                concat!(
                    "+ACI-The sayings of Confucius,+ACI- James R. Ware, trans.  +U/BTFw-:\n",
                    "+ZYeB9FH6ckh5Pg-, 1980.\n",
                    "+Vttm+E6UfZM-, +W4tRQ066bOg-, +UxdOrA-:  +Ti1XC2b4Xpc-, 1990."
                ),
                concat!(
                    "\"The sayings of Confucius,\" James R. Ware, trans.  台北:\n",
                    "文致出版社, 1980.\n",
                    "四書五經, 宋元人注, 北京:  中國書店, 1990."
                ),
            ),
        ];

        for (input, expected) in inputs {
            assert_eq!(Charset::Utf7.decode(input.as_bytes()), expected);
        }
        for (input, expected) in [
            ("+AKM.", "\u{a3}."),
            ("x +AKM y", "x \u{a3} y"),
            ("+AKM, then", "\u{a3}, then"),
            ("+AKM\r\nnext", "\u{a3}\r\nnext"),
            ("+AKM-- dash", "\u{a3}- dash"),
            ("1 +- 1", "1 + 1"),
            ("+!", "+!"),
            ("+A.", "\u{fffd}."),
            ("caf\u{e9}", "caf\u{c3}\u{a9}"),
            ("Hi +AKM", "Hi "),
            ("+A-", "\u{fffd}"),
            ("+2D3eAA-", "\u{1f600}"),
            ("+2D0-", "\u{fffd}"),
        ] {
            assert_eq!(Charset::Utf7.decode(input.as_bytes()), expected, "{input}");
        }
    }

    #[cfg(feature = "full_encoding")]
    #[test]
    fn encoding_rs_charsets_drop_a_utf8_bom() {
        for (bytes, expected) in [
            (&b"\xef\xbb\xbf"[..], ""),
            (b"\xef\xbb\xbfabc\xd0\x96", "abc\u{416}"),
        ] {
            for charset in [Charset::Ibm866, Charset::ShiftJis, Charset::XMacCyrillic] {
                let decoded = charset.decode(bytes);
                assert!(matches!(decoded, Cow::Borrowed(_)), "{charset:?} {bytes:?}");
                assert_eq!(decoded, expected, "{charset:?} {bytes:?}");
                assert_eq!(charset.decode_owned(bytes.to_vec()), expected);
                let mut appended = String::from("<");
                charset.decode_append(bytes, &mut appended);
                assert_eq!(appended.strip_prefix('<'), Some(expected));
                assert_eq!(charset.decode_checked(bytes).0, expected);
            }
        }
    }

    #[test]
    fn utf16_dangling_byte_is_replaced() {
        for (charset, bytes, expected) in [
            (Charset::Utf16Le, &b"a\x00b\x00c"[..], "ab\u{fffd}"),
            (Charset::Utf16Be, b"\x00a\x00b\x00", "ab\u{fffd}"),
            (Charset::Utf16, b"\xff\xfea\x00b", "a\u{fffd}"),
            (Charset::Utf16, b"\xfe\xff\x00a\x00", "a\u{fffd}"),
            (Charset::Utf16Le, b"\x00\xd8a", "\u{fffd}"),
            (Charset::Utf16Be, b"\xd8\x00\x00a\x00", "\u{fffd}a\u{fffd}"),
            (Charset::Utf16Le, b"x", "\u{fffd}"),
            (Charset::Utf16Le, b"a\x00", "a"),
        ] {
            assert_eq!(charset.decode(bytes), expected, "{charset:?} {bytes:?}");
            assert_eq!(charset.decode_owned(bytes.to_vec()), expected);
            let mut appended = String::from("<");
            charset.decode_append(bytes, &mut appended);
            assert_eq!(appended.strip_prefix('<'), Some(expected));
            let (checked, malformed) = charset.decode_checked(bytes);
            assert_eq!(checked, expected, "{charset:?} {bytes:?}");
            assert_eq!(malformed, expected.contains('\u{fffd}'));
        }
    }
}
