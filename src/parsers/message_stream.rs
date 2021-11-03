/*
 * Copyright Stalwart Labs, Minter Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::{borrow::Cow, cell::Cell};

pub struct MessageStream<'x> {
    pub data: &'x [u8],
    pub pos: Cell<usize>,
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &'x [u8]) -> MessageStream<'x> {
        MessageStream {
            data,
            pos: 0.into(),
        }
    }

    #[inline(always)]
    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&'x [u8]> {
        self.data.get(from..to)
    }

    pub fn get_string(&self, from: usize, to: usize) -> Option<Cow<'x, str>> {
        String::from_utf8_lossy(self.data.get(from..to)?).into()
    }

    #[inline(always)]
    pub fn is_eof(&self) -> bool {
        self.pos.get() >= self.data.len()
    }

    #[inline(always)]
    pub fn set_pos(&self, pos: usize) {
        self.pos.set(pos);
    }

    #[inline(always)]
    pub fn get_pos(&self) -> usize {
        self.pos.get()
    }

    #[inline(always)]
    pub fn skip_crlf(&self) {
        let mut pos = self.pos.get();

        for ch in self.data[pos..].iter() {
            match ch {
                b'\r' | b' ' | b'\t' => pos += 1,
                b'\n' => {
                    self.pos.set(pos + 1);
                    break;
                }
                _ => break,
            }
        }
    }
}
