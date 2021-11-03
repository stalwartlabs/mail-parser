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
    pub fn next(&self) -> Option<&'x u8> {
        let pos = self.pos.get();
        let result = self.data.get(pos);
        self.pos.set(pos + 1);
        result
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<&'x u8> {
        self.data.get(self.pos.get())
    }

    #[inline(always)]
    pub fn advance(&self, pos: usize) {
        self.pos.set(self.pos.get() + pos)
    }

    pub fn seek_next_part(&self, boundary: &[u8]) -> bool {
        if !boundary.is_empty() {
            let mut pos = self.pos.get();

            let mut match_count = 0;

            for ch in self.data[pos..].iter() {
                pos += 1;

                if ch == &boundary[match_count] {
                    match_count += 1;
                    if match_count == boundary.len() {
                        self.pos.set(pos);
                        return true;
                    } else {
                        continue;
                    }
                } else if match_count > 0 {
                    if ch == &boundary[0] {
                        match_count = 1;
                        continue;
                    } else {
                        match_count = 0;
                    }
                }
            }
        }

        false
    }

    pub fn get_bytes_to_boundary(&self, boundary: &[u8]) -> (bool, Option<Cow<'x, [u8]>>) {
        let mut pos = self.pos.get();

        let start_pos = pos;

        return if !boundary.is_empty() {
            let mut match_count = 0;

            for ch in self.data[pos..].iter() {
                pos += 1;

                if ch == &boundary[match_count] {
                    match_count += 1;
                    if match_count == boundary.len() {
                        if self.is_boundary_end(pos) {
                            let match_pos = pos - match_count;
                            self.pos.set(pos);

                            return (
                                true,
                                if start_pos < match_pos {
                                    Cow::from(&self.data[start_pos..match_pos]).into()
                                } else {
                                    None
                                },
                            );
                        } else {
                            match_count = 0;
                        }
                    }
                    continue;
                } else if match_count > 0 {
                    if ch == &boundary[0] {
                        match_count = 1;
                        continue;
                    } else {
                        match_count = 0;
                    }
                }
            }

            (false, None)
        } else if pos < self.data.len() {
            self.pos.set(self.data.len());
            (true, Cow::from(&self.data[start_pos..]).into())
        } else {
            (false, None)
        };
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

    #[inline(always)]
    pub fn skip_byte(&self, ch: &u8) -> bool {
        let pos = self.pos.get();

        if self.data.get(pos) == Some(ch) {
            self.pos.set(pos + 1);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn is_boundary_end(&self, pos: usize) -> bool {
        matches!(
            self.data.get(pos..),
            Some([b'\n' | b'\r' | b' ' | b'\t', ..]) | Some([b'-', b'-', ..]) | Some([]) | None
        )
    }

    pub fn skip_multipart_end(&self) -> bool {
        let pos = self.pos.get();

        match self.data.get(pos..pos + 2) {
            Some(b"--") => {
                if let Some(byte) = self.data.get(pos + 2) {
                    if !(*byte).is_ascii_whitespace() {
                        return false;
                    }
                }
                self.pos.set(pos + 2);
                true
            }
            _ => false,
        }
    }

    pub fn rewind(&self, r: usize) {
        self.pos.set(self.pos.get() - r);
    }
}
