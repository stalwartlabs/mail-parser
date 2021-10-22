use crate::parsers::message_stream::MessageStream;

use self::decoder::Decoder;

pub mod base64;
pub mod buffer_writer;
pub mod bytes;
pub mod charsets;
pub mod decoder;
pub mod encoded_word;
pub mod hex;
pub mod quoted_printable;

pub type DecodeFnc<'x> = fn(&MessageStream<'x>, &[u8], bool, &mut dyn Decoder) -> bool;
