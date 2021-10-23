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

    pub fn match_bytes(&self, start_pos: usize, bytes: &[u8]) -> bool {
        unsafe {
            (*self.data.get())
                .get(start_pos..start_pos + bytes.len())
                .unwrap_or(&[])
                == bytes
        }
    }

    pub fn seek_bytes(&self, bytes: &[u8]) -> bool {
        if !bytes.is_empty() {
            unsafe {
                let cur_pos = &mut *self.pos.get();
                let data = &mut *self.data.get();
                let mut pos = *cur_pos;
                let mut match_count = 0;

                for ch in (*data)[*cur_pos..].iter() {
                    pos += 1;

                    if ch == bytes.get_unchecked(match_count) {
                        match_count += 1;
                        if match_count == bytes.len() {
                            *cur_pos = pos;
                            return true;
                        } else {
                            continue;
                        }
                    } else if match_count > 0 {
                        if ch == bytes.get_unchecked(0) {
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

    pub fn get_bytes_bounded(&self, boundary: &[u8]) -> (bool, bool, Option<&'x [u8]>) {
        unsafe {
            let data = &mut *self.data.get();

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

                (false, false, None)
            } else if *stream_pos < (*data).len() {
                *stream_pos = (*data).len();
                (true, false, (*data).get(start_pos..))
            } else {
                (false, false, None)
            };
        }
    }

    pub fn skip_crlf(&self) {
        unsafe {
            let cur_pos = &mut *self.pos.get();
            let data = &mut *self.data.get();
            let mut pos = *cur_pos;

            for ch in (*data)[*cur_pos..].iter() {
                match ch {
                    b'\r' => pos += 1,
                    b'\n' => {
                        *cur_pos = pos + 1;
                        break;
                    }
                    _ => break,
                }
            }
        }
    }

    pub fn skip_byte(&self, ch: &u8) -> bool {
        unsafe {
            let pos = &mut *self.pos.get();
            let data = &mut *self.data.get();

            if *pos < (*data).len() && (*data).get_unchecked(*pos) == ch {
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
            let data = &mut *self.data.get();
            let to = *pos + chs.len();

            match (*data).get(*pos..to) {
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
