use std::borrow::Cow;

use encoding_rs::*;

fn multi_byte_decoder(mut decoder: Decoder, bytes: &[u8]) -> Cow<str> {
    let mut result = String::with_capacity(bytes.len() * 3);

    if let (CoderResult::OutputFull, _, _) = decoder.decode_to_string(bytes, &mut result, true) {
        debug_assert!(false, "String full while decoding")
    }

    result.shrink_to_fit();
    result.into()
}

pub fn decoder_shift_jis(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(SHIFT_JIS.new_decoder(), bytes)
}

pub fn decoder_big5(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(BIG5.new_decoder(), bytes)
}

pub fn decoder_euc_jp(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(EUC_JP.new_decoder(), bytes)
}

pub fn decoder_euc_kr(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(EUC_KR.new_decoder(), bytes)
}

pub fn decoder_gb18030(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(GB18030.new_decoder(), bytes)
}

pub fn decoder_gbk(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(GBK.new_decoder(), bytes)
}

pub fn decoder_iso2022_jp(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(ISO_2022_JP.new_decoder(), bytes)
}

pub fn decoder_utf16_be(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(UTF_16BE.new_decoder(), bytes)
}

pub fn decoder_utf16_le(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(UTF_16LE.new_decoder(), bytes)
}

pub fn decoder_windows874(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(WINDOWS_874.new_decoder(), bytes)
}

pub fn decoder_ibm866(bytes: &[u8]) -> Cow<str> {
    multi_byte_decoder(IBM866.new_decoder(), bytes)
}
