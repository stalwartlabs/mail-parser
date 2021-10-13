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
    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&[u8]> {
        self.data.get(from..to)
    }

    pub fn get_string(&self, from: usize, to: usize, utf8_valid: bool) -> Option<Cow<str>> {
        let bytes = self.data.get(from..to)?;

        Some(if utf8_valid {
            (unsafe { std::str::from_utf8_unchecked(bytes) }).into()
        } else {
            String::from_utf8_lossy(bytes)
        })
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
    pub fn next(&self) -> Option<&u8> {
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
    pub fn peek(&self) -> Option<&u8> {
        unsafe { self.data.get(*self.pos.get()) }
    }

    #[inline(always)]
    pub fn advance(&self, pos: usize) {
        unsafe {
            *self.pos.get() += pos;
        }
    }

    pub fn match_bytes(&self, start_pos: usize, bytes: &[u8]) -> bool {
        self.data.get(start_pos..start_pos + bytes.len()).unwrap_or(&[]) == bytes
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
