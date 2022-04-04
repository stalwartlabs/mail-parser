/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#[cfg(feature = "full_encoding")]
use encoding_rs::*;

#[cfg(feature = "full_encoding")]
fn multi_byte_decoder(mut decoder: Decoder, bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 3);

    if let (CoderResult::OutputFull, _, _) = decoder.decode_to_string(bytes, &mut result, true) {
        debug_assert!(false, "String full while decoding.")
    }

    result.shrink_to_fit();
    result
}

pub fn decoder_shift_jis(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(SHIFT_JIS.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_big5(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(BIG5.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_euc_jp(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(EUC_JP.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_euc_kr(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(EUC_KR.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_gb18030(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(GB18030.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_gbk(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(GBK.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_iso2022_jp(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(ISO_2022_JP.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_windows874(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(WINDOWS_874.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn decoder_ibm866(bytes: &[u8]) -> String {
    #[cfg(feature = "full_encoding")]
    {
        multi_byte_decoder(IBM866.new_decoder(), bytes)
    }

    #[cfg(not(feature = "full_encoding"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}
