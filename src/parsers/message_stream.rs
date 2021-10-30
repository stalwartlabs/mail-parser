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

use std::{borrow::Cow, cell::UnsafeCell};

pub struct MessageStream<'x> {
    pub data: UnsafeCell<&'x mut [u8]>,
    pub pos: UnsafeCell<usize>,
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &'x mut [u8]) -> MessageStream<'x> {
        MessageStream {
            data: data.into(),
            pos: 0.into(),
        }
    }

    #[inline(always)]
    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&'x [u8]> {
        unsafe { (*self.data.get()).get(from..to) }
    }

    pub fn get_string(&self, from: usize, to: usize, utf8_valid: bool) -> Option<Cow<'x, str>> {
        unsafe {
            if utf8_valid {
                Cow::from(std::str::from_utf8_unchecked(
                    (*self.data.get()).get(from..to)?,
                ))
                .into()
            } else {
                String::from_utf8_lossy((*self.data.get()).get(from..to)?).into()
            }
        }
    }

    #[inline(always)]
    pub fn is_eof(&self) -> bool {
        unsafe { *self.pos.get() >= (*self.data.get()).len() }
    }

    #[inline(always)]
    pub fn set_pos(&self, pos: usize) {
        unsafe {
            *self.pos.get() = pos;
        }
    }

    #[inline(always)]
    pub fn get_pos(&self) -> usize {
        unsafe { *self.pos.get() }
    }

    #[inline(always)]
    pub fn next(&self) -> Option<&'x u8> {
        unsafe {
            let pos = &mut *self.pos.get();
            let data = &mut *self.data.get();

            if *pos < data.len() {
                let result = data.get_unchecked(*pos);
                *pos += 1;
                Some(result)
            } else {
                None
            }
        }
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<&'x u8> {
        unsafe { (*self.data.get()).get(*self.pos.get()) }
    }

    #[inline(always)]
    pub fn advance(&self, pos: usize) {
        unsafe {
            *self.pos.get() += pos;
        }
    }

    pub fn seek_next_part(&self, boundary: &[u8]) -> bool {
        if !boundary.is_empty() {
            unsafe {
                let cur_pos = &mut *self.pos.get();
                let data = &mut *self.data.get();
                let mut pos = *cur_pos;
                let mut match_count = 0;

                for ch in (*data)[*cur_pos..].iter() {
                    pos += 1;

                    if ch == boundary.get_unchecked(match_count) {
                        match_count += 1;
                        if match_count == boundary.len() {
                            *cur_pos = pos;
                            return true;
                        } else {
                            continue;
                        }
                    } else if match_count > 0 {
                        if ch == boundary.get_unchecked(0) {
                            match_count = 1;
                            continue;
                        } else {
                            match_count = 0;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn get_bytes_to_boundary(&self, boundary: &[u8]) -> (bool, bool, Option<&'x [u8]>) {
        unsafe {
            let mut data = &mut *self.data.get();

            let stream_pos = &mut *self.pos.get();
            let start_pos = *stream_pos;

            return if !boundary.is_empty() {
                let mut pos = *stream_pos;
                let mut match_count = 0;
                let mut is_utf8_safe = true;

                for ch in (*data)[pos..].iter() {
                    pos += 1;

                    if is_utf8_safe && *ch > 0x7f {
                        is_utf8_safe = false;
                    }

                    if ch == boundary.get_unchecked(match_count) {
                        match_count += 1;
                        if match_count == boundary.len() {
                            let is_boundary_end = self.is_boundary_end(pos);
                            data = &mut *self.data.get(); // Avoid violating the Stacked Borrows rules

                            if is_boundary_end {
                                let match_pos = pos - match_count;
                                *stream_pos = pos;

                                return (
                                    true,
                                    is_utf8_safe,
                                    if start_pos < match_pos {
                                        (*data).get(start_pos..match_pos)
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
                        if ch == boundary.get_unchecked(0) {
                            match_count = 1;
                            continue;
                        } else {
                            match_count = 0;
                        }
                    }
                }

                (false, false, None)
            } else if *stream_pos < (*data).len() {
                *stream_pos = (*data).len();
                (true, false, (*data).get(start_pos..))
            } else {
                (false, false, None)
            };
        }
    }

    #[inline(always)]
    pub fn skip_crlf(&self) {
        unsafe {
            let cur_pos = &mut *self.pos.get();
            let data = &mut *self.data.get();
            let mut pos = *cur_pos;

            for ch in (*data)[*cur_pos..].iter() {
                match ch {
                    b'\r' | b' ' | b'\t' => pos += 1,
                    b'\n' => {
                        *cur_pos = pos + 1;
                        break;
                    }
                    _ => break,
                }
            }
        }
    }

    #[inline(always)]
    pub fn skip_byte(&self, ch: &u8) -> bool {
        unsafe {
            let pos = &mut *self.pos.get();
            let data = &mut *self.data.get();

            if (*data).get(*pos) == Some(ch) {
                *pos += 1;
                true
            } else {
                false
            }
        }
    }

    #[inline(always)]
    pub fn is_boundary_end(&self, pos: usize) -> bool {
        matches!(
            unsafe { (*self.data.get()).get(pos..) },
            Some([b'\n' | b'\r' | b' ' | b'\t', ..]) | Some([b'-', b'-', ..]) | Some([]) | None
        )
    }

    pub fn skip_multipart_end(&self) -> bool {
        unsafe {
            let pos = &mut *self.pos.get();

            match (*self.data.get()).get(*pos..*pos + 2) {
                Some(b"--") => {
                    if let Some(byte) = (*self.data.get()).get(*pos + 2) {
                        if !(*byte).is_ascii_whitespace() {
                            return false;
                        }
                    }
                    *pos += 2;
                    true
                }
                _ => false,
            }
        }
    }

    pub fn rewind(&self, r: usize) {
        unsafe {
            *self.pos.get() -= r;
        }
    }
}
