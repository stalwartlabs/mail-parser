use encoding_rs::*;

use crate::decoders::decoder::Decoder;

pub struct MultiByteDecoder<'x> {
    decoder: encoding_rs::Decoder,
    buf: &'x mut [u8],
    pos: usize,
}

impl<'x> Decoder for MultiByteDecoder<'x> {
    fn write_byte(&mut self, byte: &u8) -> bool {
        self.write_bytes(&[*byte])
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        let (result, read, written, _) =
            self.decoder
                .decode_to_utf8(bytes, &mut self.buf[self.pos..], false);

        debug_assert_eq!(read, bytes.len());

        self.pos += written;

        return match result {
            CoderResult::InputEmpty => true,
            CoderResult::OutputFull => false,
        };
    }

    fn len(&self) -> usize {
        self.pos
    }

    fn is_utf8_safe(&self) -> bool {
        true
    }
}

impl MultiByteDecoder<'_> {
    pub fn get_shift_jis<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: SHIFT_JIS.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_big5<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: BIG5.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_euc_jp<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: EUC_JP.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_euc_kr<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: EUC_KR.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_gb18030<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: GB18030.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_gbk<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: GBK.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_iso2022_jp<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: ISO_2022_JP.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_utf16_be<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: UTF_16BE.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_utf16_le<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: UTF_16LE.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_windows874<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: WINDOWS_874.new_decoder().into(),
            buf,
            pos: 0,
        })
    }

    pub fn get_ibm866<'x>(buf: &'x mut [u8]) -> Box<dyn Decoder + 'x> {
        Box::new(MultiByteDecoder {
            decoder: IBM866.new_decoder().into(),
            buf,
            pos: 0,
        })
    }
}
