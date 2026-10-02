/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::borrow::Cow;

#[cfg(feature = "full_encoding")]
use encoding_rs::{
    BIG5, CoderResult, Decoder, EUC_JP, EUC_KR, Encoding, GB18030, GBK, IBM866, ISO_2022_JP,
    REPLACEMENT, SHIFT_JIS, X_MAC_CYRILLIC, X_USER_DEFINED,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MultiByte {
    ShiftJis,
    Big5,
    EucJp,
    EucKr,
    Gb18030,
    Gbk,
    Iso2022Jp,
    Ibm866,
    XMacCyrillic,
    XUserDefined,
    Replacement,
}

#[cfg(feature = "full_encoding")]
const MIN_ROOM: usize = 16;

#[cfg(feature = "full_encoding")]
impl MultiByte {
    fn encoding(self) -> &'static Encoding {
        match self {
            MultiByte::ShiftJis => SHIFT_JIS,
            MultiByte::Big5 => BIG5,
            MultiByte::EucJp => EUC_JP,
            MultiByte::EucKr => EUC_KR,
            MultiByte::Gb18030 => GB18030,
            MultiByte::Gbk => GBK,
            MultiByte::Iso2022Jp => ISO_2022_JP,
            MultiByte::Ibm866 => IBM866,
            MultiByte::XMacCyrillic => X_MAC_CYRILLIC,
            MultiByte::XUserDefined => X_USER_DEFINED,
            MultiByte::Replacement => REPLACEMENT,
        }
    }

    pub(super) fn decode(self, bytes: &[u8]) -> Cow<'_, str> {
        self.encoding().decode(bytes).0
    }

    pub(super) fn decode_checked(self, bytes: &[u8]) -> (Cow<'_, str>, bool) {
        let (text, _, malformed) = self.encoding().decode(bytes);
        (text, malformed)
    }

    pub(super) fn decode_append(self, bytes: &[u8], out: &mut String) {
        if !matches!(self, MultiByte::Iso2022Jp | MultiByte::Replacement)
            && let (ascii, []) = super::ascii::split_ascii(bytes)
        {
            out.push_str(ascii);
        } else {
            decode_with(self.encoding().new_decoder(), bytes, out);
        }
    }

    pub(super) fn decode_prefix(self, bytes: &[u8]) -> Cow<'_, str> {
        if !matches!(self, MultiByte::Iso2022Jp | MultiByte::Replacement)
            && let (ascii, []) = super::ascii::split_ascii(bytes)
        {
            return Cow::Borrowed(ascii);
        }
        let mut decoder = self.encoding().new_decoder();
        let mut out = String::with_capacity(
            decoder
                .max_utf8_buffer_length(bytes.len())
                .unwrap_or(bytes.len())
                .max(MIN_ROOM),
        );
        let mut rest = bytes;
        loop {
            let (result, read, _) = decoder.decode_to_string(rest, &mut out, false);
            rest = rest.get(read..).unwrap_or_default();
            if result == CoderResult::InputEmpty {
                break;
            }
            out.reserve(
                decoder
                    .max_utf8_buffer_length(rest.len())
                    .unwrap_or(rest.len())
                    .max(MIN_ROOM),
            );
        }
        Cow::Owned(out)
    }
}

#[cfg(feature = "full_encoding")]
pub(super) fn decode_with(mut decoder: Decoder, mut bytes: &[u8], out: &mut String) {
    loop {
        out.reserve(
            decoder
                .max_utf8_buffer_length(bytes.len())
                .unwrap_or(bytes.len())
                .max(MIN_ROOM),
        );
        let (result, read, _) = decoder.decode_to_string(bytes, out, true);
        bytes = bytes.get(read..).unwrap_or_default();
        if result == CoderResult::InputEmpty {
            break;
        }
    }
}

#[cfg(not(feature = "full_encoding"))]
impl MultiByte {
    pub(super) fn decode_prefix(self, bytes: &[u8]) -> Cow<'_, str> {
        super::utf8::decode(super::utf8::complete_prefix(bytes))
    }

    pub(super) fn decode(self, bytes: &[u8]) -> Cow<'_, str> {
        super::utf8::decode(bytes)
    }

    pub(super) fn decode_checked(self, bytes: &[u8]) -> (Cow<'_, str>, bool) {
        super::utf8::decode_checked(bytes)
    }

    pub(super) fn decode_append(self, bytes: &[u8], out: &mut String) {
        super::utf8::decode_append(bytes, out);
    }
}
