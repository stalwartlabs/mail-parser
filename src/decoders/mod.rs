pub mod quoted_printable;
pub mod base64;
pub mod charsets;

pub enum DecoderResult<'x> {
    Byte(u8),
    ByteArray(&'x [u8]),
    NeedData,
    Error
}

pub trait Decoder {
    fn ingest(&mut self, ch: u8) -> DecoderResult;
}
