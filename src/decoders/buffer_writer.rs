use super::Writer;

pub struct BufferWriter {
    buf: Vec<u8>,
}

impl BufferWriter {
    pub fn with_capacity(capacity: usize) -> BufferWriter {
        BufferWriter {
            buf: Vec::with_capacity(capacity),
        }
    }
}

impl Writer for BufferWriter {
    fn write_byte(&mut self, byte: &u8) -> bool {
        self.buf.push(*byte);
        true
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        self.buf.extend_from_slice(bytes);
        true
    }

    fn get_bytes(&mut self) -> Option<Box<[u8]>> {
        if !self.buf.is_empty() {
            Some(std::mem::take(&mut self.buf).into_boxed_slice())
        } else {
            None
        }
    }
}
