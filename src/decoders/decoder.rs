pub trait Decoder {
    fn write_byte(&mut self, byte: &u8) -> bool;
    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        for byte in bytes {
            if !self.write_byte(byte) {
                return false;
            }
        }
        true
    }
    fn len(&self) -> usize;
    fn is_utf8_safe(&self) -> bool;
}

pub struct RawDecoder<'x> {
    buf: &'x mut [u8],
    pos: usize,
}

impl<'x> RawDecoder<'x> {
    pub fn new(buf: &'x mut [u8]) -> RawDecoder<'x> {
        RawDecoder { buf, pos: 0 }
    }
}

impl<'x> Decoder for RawDecoder<'x> {
    fn write_byte(&mut self, byte: &u8) -> bool {
        if let Some(b) = self.buf.get_mut(self.pos) {
            *b = *byte;
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        if !bytes.is_empty() {
            let new_pos = bytes.len() + self.pos;
            if let Some(b) = self.buf.get_mut(self.pos..new_pos) {
                b.copy_from_slice(bytes);
                self.pos = new_pos;
                return true;
            }
        }
        false
    }

    fn len(&self) -> usize {
        self.pos
    }

    fn is_utf8_safe(&self) -> bool {
        false
    }
}
