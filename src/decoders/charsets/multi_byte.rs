use std::cell::UnsafeCell;

use encoding_rs::*;

use crate::decoders::{buffer_writer::BufferWriter, Writer};

pub struct MultiByteDecoder<'x> {
    decoder: UnsafeCell<Decoder>,
    buf: &'x BufferWriter,
}

impl<'x> Writer for MultiByteDecoder<'x> {
    fn write_byte(&self, byte: &u8) -> bool {
        self.write_bytes(&[*byte])
    }

    fn write_bytes(&self, bytes: &[u8]) -> bool {
        if let Some(buf) = self.buf.get_buf_mut() {
            let (result, read, written, _) =
                unsafe { (*self.decoder.get()).decode_to_utf8(bytes, buf, false) };

            debug_assert_eq!(read, bytes.len());

            if written > 0 {
                self.buf.advance_tail(written);
            }

            return match result {
                CoderResult::InputEmpty => true,
                CoderResult::OutputFull => false,
            };
        }
        false
    }
}

impl MultiByteDecoder<'_> {
    pub fn get_shift_jis<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: SHIFT_JIS.new_decoder().into(),
            buf,
        })
    }

    pub fn get_big5<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: BIG5.new_decoder().into(),
            buf,
        })
    }

    pub fn get_euc_jp<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: EUC_JP.new_decoder().into(),
            buf,
        })
    }

    pub fn get_euc_kr<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: EUC_KR.new_decoder().into(),
            buf,
        })
    }

    pub fn get_gb18030<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: GB18030.new_decoder().into(),
            buf,
        })
    }

    pub fn get_gbk<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: GBK.new_decoder().into(),
            buf,
        })
    }

    pub fn get_iso2022_jp<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: ISO_2022_JP.new_decoder().into(),
            buf,
        })
    }

    pub fn get_utf16_be<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: UTF_16BE.new_decoder().into(),
            buf,
        })
    }

    pub fn get_utf16_le<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: UTF_16LE.new_decoder().into(),
            buf,
        })
    }

    pub fn get_windows874<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: WINDOWS_874.new_decoder().into(),
            buf,
        })
    }

    pub fn get_ibm866<'x>(buf: &'x BufferWriter) -> Box<dyn Writer + 'x> {
        Box::new(MultiByteDecoder {
            decoder: IBM866.new_decoder().into(),
            buf,
        })
    }
}
