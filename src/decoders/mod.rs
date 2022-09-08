/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use crate::parsers::message::MessageStream;

pub mod base64;
pub mod charsets;
pub mod encoded_word;
pub mod hex;
pub mod html;
pub mod quoted_printable;

pub type DecodeFnc<'x> = fn(&MessageStream<'x>, usize, &[u8], bool) -> (usize, DecodeResult);

#[derive(Debug)]
pub enum DecodeResult {
    Owned(Vec<u8>),
    Borrowed((usize, usize)),
    Empty,
}
