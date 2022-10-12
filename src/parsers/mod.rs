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

use std::{iter::Peekable, slice::Iter};

pub mod fields;
pub mod header;
pub mod message;
pub mod mime;
pub mod preview;

/*pub struct MessageStream<'x> {
    data: &'x [u8],
    iter: Peekable<Iter<'x, u8>>,
    pos: usize,
    restore_pos: usize,
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &'x [u8]) -> MessageStream<'x> {
        MessageStream {
            data,
            iter: data.iter().peekable(),
            pos: 0,
            restore_pos: 0,
        }
    }

    #[inline(always)]
    pub fn peek(&mut self) -> Option<&&u8> {
        self.iter.peek()
    }

    #[inline(always)]
    pub fn offset(&self) -> usize {
        self.pos
    }

    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    #[inline(always)]
    pub fn checkpoint(&mut self) {
        self.restore_pos = self.pos;
    }

    #[inline(always)]
    pub fn restore(&mut self) {
        self.iter = self.data[self.restore_pos..].iter().peekable();
        self.pos = self.restore_pos;
        self.restore_pos = 0;
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.restore_pos = 0;
    }
}

impl<'x> Iterator for MessageStream<'x> {
    type Item = &'x u8;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.iter.next()
    }
}
*/
