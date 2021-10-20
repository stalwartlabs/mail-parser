use encoding_rs::*;

use crate::decoders::Writer;

pub struct MultiByteDecoder {
    decoder: Decoder,
    result: String,
}

impl Writer for MultiByteDecoder {
    fn write_byte(&mut self, byte: &u8) -> bool {
        self.write_bytes(&[*byte])
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        match self
            .decoder
            .decode_to_string(bytes, &mut self.result, false)
        {
            (encoding_rs::CoderResult::OutputFull, _, _) => false,
            (_, _, _) => true,
        }
    }

    fn get_string(&mut self) -> Option<String> {
        if !self.result.is_empty() {
            Some(std::mem::take(&mut self.result))
        } else {
            None
        }
    }

    fn get_bytes(&mut self) -> Option<Box<[u8]>> {
        None
    }

    fn is_empty(&self) -> bool {
        self.result.is_empty()
    }    
}

impl MultiByteDecoder {
    pub fn get_shift_jis(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: SHIFT_JIS.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_big5(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: BIG5.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_euc_jp(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: EUC_JP.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_euc_kr(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: EUC_KR.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_gb18030(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: GB18030.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_gbk(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: GBK.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_iso2022_jp(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: ISO_2022_JP.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_utf16_be(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: UTF_16BE.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_utf16_le(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: UTF_16LE.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_windows874(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: WINDOWS_874.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }

    pub fn get_ibm866(capacity: usize) -> Box<dyn Writer> {
        Box::new(MultiByteDecoder {
            decoder: IBM866.new_decoder(),
            result: String::with_capacity(capacity),
        })
    }
}
