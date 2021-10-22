use std::{borrow::Cow, cell::UnsafeCell};

pub struct MessageStream<'x> {
    pub data: &'x [u8],
    pos: UnsafeCell<usize>,
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

    pub fn get_string(&self, from: usize, to: usize, utf8_valid: bool) -> Option<Cow<'x, str>> {
        let bytes = self.data.get(from..to)?;

        Some(if utf8_valid {
            (unsafe { std::str::from_utf8_unchecked(bytes) }).into()
        } else {
            String::from_utf8_lossy(bytes)
        })
    }

    #[inline(always)]
    pub fn is_eof(&self) -> bool {
        unsafe { *self.pos.get() >= self.data.len() }
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

            if *pos < self.data.len() {
                let result = self.data.get_unchecked(*pos);
                *pos += 1;
                Some(result)
            } else {
                None
            }
        }
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<&'x u8> {
        unsafe { self.data.get(*self.pos.get()) }
    }

    #[inline(always)]
    pub fn advance(&self, pos: usize) {
        unsafe {
            *self.pos.get() += pos;
        }
    }

    pub fn match_bytes(&self, start_pos: usize, bytes: &[u8]) -> bool {
        self.data
            .get(start_pos..start_pos + bytes.len())
            .unwrap_or(&[])
            == bytes
    }

    pub fn seek_bytes(&self, bytes: &[u8]) -> bool {
        let mut pos = self.get_pos();
        let mut match_count = 0;

        if !bytes.is_empty() {
            for ch in self.data[pos..].iter() {
                pos += 1;

                if ch == unsafe { bytes.get_unchecked(match_count) } {
                    match_count += 1;
                    if match_count == bytes.len() {
                        self.set_pos(pos);
                        return true;
                    } else {
                        continue;
                    }
                } else if match_count > 0 {
                    if ch == unsafe { bytes.get_unchecked(0) } {
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

    pub fn skip_spaces(&self) -> bool {
        let mut pos = self.get_pos();
        for ch in self.data[pos..].iter() {
            if !ch.is_ascii_whitespace() {
                break;
            } else {
                pos += 1;
            }
        }
        self.set_pos(pos);
        pos < self.data.len()
    }

    pub fn skip_crlf(&self) {
        let mut pos = self.get_pos();
        for ch in self.data[pos..].iter() {
            match ch {
                b'\r' => pos += 1,
                b'\n' => {
                    self.set_pos(pos + 1);
                    break;
                }
                _ => break,
            }
        }
    }

    pub fn skip_byte(&self, ch: &u8) -> bool {
        unsafe {
            let pos = &mut *self.pos.get();

            if *pos < self.data.len() && self.data.get_unchecked(*pos) == ch {
                *pos += 1;
                true
            } else {
                false
            }
        }
    }

    pub fn skip_bytes(&self, chs: &[u8]) -> bool {
        unsafe {
            let pos = &mut *self.pos.get();
            let to = *pos + chs.len();

            match self.data.get(*pos..to) {
                Some(bytes) if bytes == chs => {
                    *pos = to;
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
