use super::CharsetDecoder;
use encoding_rs::*;

pub struct MultiByteDecoder {
    decoder: Decoder,
    result: String
}

impl CharsetDecoder for MultiByteDecoder {
    fn ingest(&mut self, ch: u8) -> () {
        self.ingest_slice(&[ch]);
    }

    fn ingest_slice(&mut self, chs: &[u8]) -> () {
        match self.decoder.decode_to_string(chs, &mut self.result, false) {
            (_, _, _) => (),
        }
    }

    fn to_string(&self) -> Option<&str> {
        if self.result.len() > 0 {
            Some(&self.result)
        } else {
            None
        }
    }

    fn needs_slice(&self) -> bool {
        true
    }
}

impl MultiByteDecoder {
    pub fn get_shift_jis(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: SHIFT_JIS.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_big5(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: BIG5.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_euc_jp(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: EUC_JP.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_euc_kr(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: EUC_KR.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_gb18030(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: GB18030.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_gbk(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: GBK.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_iso2022_jp(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: ISO_2022_JP.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_utf16_be(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: UTF_16BE.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

    pub fn get_utf16_le(capacity: usize) -> MultiByteDecoder {
        MultiByteDecoder {
            decoder: UTF_16LE.new_decoder(),
            result: String::with_capacity(capacity),
        }
    }

}
