pub mod base64;
pub mod buffer_writer;
pub mod charsets;
pub mod encoded_word;
pub mod hex;
pub mod quoted_printable;
pub mod bytes;

pub trait Writer {
    fn write_byte(&self, byte: &u8) -> bool;
    fn write_bytes(&self, bytes: &[u8]) -> bool {
        for byte in bytes {
            if !self.write_byte(byte) {
                return false;
            }
        }
        true
    }
}
