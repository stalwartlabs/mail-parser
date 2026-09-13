/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{iter::Peekable, ops::Range, slice::Iter};

pub mod fields;
pub mod header;
pub mod message;
pub mod mime;
pub mod preview;

pub struct MessageStream<'x> {
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
        std::cmp::min(self.pos, self.data.len())
    }

    #[inline(always)]
    pub(crate) fn is_at_line_start(&self) -> bool {
        self.offset() == 0 || self.data.get(self.offset() - 1) == Some(&b'\n')
    }

    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.offset()
    }

    #[inline(always)]
    pub fn checkpoint(&mut self) {
        self.restore_pos = self.offset();
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

    #[inline(always)]
    pub fn peek_bytes(&self, len: usize) -> Option<&[u8]> {
        let pos = self.offset();
        self.data.get(pos..pos + len)
    }

    #[inline(always)]
    pub fn peek_char(&mut self, ch: u8) -> bool {
        matches!(self.peek(), Some(&&ch_) if ch_ == ch)
    }

    #[inline(always)]
    pub fn skip_bytes(&mut self, len: usize) {
        self.pos += len;
        self.iter = self.data[self.pos..].iter().peekable();
    }

    #[inline(always)]
    pub fn try_skip(&mut self, bytes: &[u8]) -> bool {
        if self.peek_bytes(bytes.len()) == Some(bytes) {
            self.skip_bytes(bytes.len());
            true
        } else {
            false
        }
    }

    /// Skips a MIME boundary name after the two leading hyphens have already
    /// been consumed.
    ///
    /// MIME boundaries are only valid at the beginning of a line. The bytes
    /// following the boundary name must also be a delimiter line ending or a
    /// closing boundary marker; otherwise a body string such as
    /// `--boundary-name` would be mistaken for a boundary named `boundary`.
    #[inline(always)]
    pub(crate) fn try_skip_boundary(&mut self, boundary: &[u8], at_line_start: bool) -> bool {
        if !at_line_start
            || boundary.is_empty()
            || self.peek_bytes(boundary.len()) != Some(boundary)
        {
            return false;
        }

        let suffix = self
            .data
            .get(self.offset() + boundary.len()..)
            .unwrap_or_default();

        if !is_valid_boundary_suffix(suffix) {
            return false;
        }

        self.skip_bytes(boundary.len());
        true
    }

    #[inline(always)]
    pub fn try_skip_char(&mut self, ch: u8) -> bool {
        if self.peek_char(ch) {
            self.next();
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn bytes(&self, range: Range<usize>) -> &'x [u8] {
        self.data.get(range).unwrap_or_default()
    }

    #[inline(always)]
    pub fn seek_end(&mut self) {
        self.pos = self.data.len();
        self.iter = [][..].iter().peekable();
    }

    #[inline(always)]
    pub fn next_is_space(&mut self) -> bool {
        matches!(self.next(), Some(b' ' | b'\t'))
    }

    #[inline(always)]
    pub fn peek_next_is_space(&mut self) -> bool {
        matches!(self.peek(), Some(b' ' | b'\t'))
    }

    #[inline(always)]
    pub fn try_next_is_space(&mut self) -> bool {
        if self.peek_next_is_space() {
            self.next();
            true
        } else {
            false
        }
    }

    #[allow(clippy::len_without_is_empty)]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    pub fn is_eof(&mut self) -> bool {
        self.iter.peek().is_none()
    }
}

#[inline(always)]
fn is_valid_boundary_suffix(mut suffix: &[u8]) -> bool {
    while matches!(suffix.first(), Some(b' ' | b'\t')) {
        suffix = &suffix[1..];
    }

    if suffix.starts_with(b"--") {
        suffix = &suffix[2..];
        while matches!(suffix.first(), Some(b' ' | b'\t')) {
            suffix = &suffix[1..];
        }
    }

    suffix.is_empty() || matches!(suffix.first(), Some(b'\n')) || suffix.starts_with(b"\r\n")
}

impl<'x> Iterator for MessageStream<'x> {
    type Item = &'x u8;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.iter.next()
    }
}
