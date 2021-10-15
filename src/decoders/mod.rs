pub mod base64;
pub mod charsets;
pub mod hex;
pub mod quoted_printable;
pub mod buffer_writer;

pub trait Writer {
    fn write_byte(&mut self, byte: &u8) -> bool;
    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        for byte in bytes {
            if !self.write_byte(byte) {
                return false;
            }
        }
        true
    }

    fn get_bytes(&mut self) -> Option<Box<[u8]>>;
    fn get_string(&mut self) -> Option<String> {
        String::from_utf8(self.get_bytes()?.into()).map_or_else(|_| None, Some)
    }
}
