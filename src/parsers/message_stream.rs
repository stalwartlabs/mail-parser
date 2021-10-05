use std::borrow::{Borrow};

pub struct MessageStream<'x> {
    data: &'x mut [u8],
    read_pos: isize,
    write_pos: usize,
    write_start: usize
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &mut [u8]) -> MessageStream {
        MessageStream {
            data,
            read_pos: -1,
            write_pos: 0,
            write_start: 0
        }
    }

    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&[u8]> {
        self.data.get(from..to)
    }

    pub fn get_str_lossy(&'x self, from: usize, to: usize) -> Option<&str> {
        self.data.get(from..to).map(|bytes| unsafe { std::str::from_utf8_unchecked(bytes) })
        //self.data.get(from..to).map(|bytes| String::from_utf8_lossy(bytes).borrow() )
    }

    pub fn get_str(&'x self, from: usize, to: usize) -> Option<&str> {
        self.data.get(from..to).map(|bytes| unsafe { std::str::from_utf8_unchecked(bytes) })
    }

    #[inline(always)]
    pub fn begin_write(&mut self) {
        if self.read_pos > 0 {
            self.write_pos = self.read_pos as usize;
            self.write_start = self.read_pos as usize;
        }
    }

    #[inline(always)]
    pub fn write(&mut self, ch: u8) {
        self.data[self.write_pos] = ch;
        self.write_pos += 1;
    }

    #[inline(always)]
    pub fn write_slice(&mut self, chars: &[u8]) {
        self.data[self.write_pos..(self.write_pos+chars.len())].copy_from_slice(chars);
        self.write_pos += chars.len();        
    }

    #[inline(always)]
    pub fn end_write(&self) -> Option<&[u8]> {
        self.data.get(self.write_start..self.write_pos)
    }

    #[inline(always)]
    pub fn get_read_pos(&self) -> isize {
        self.read_pos
    }

    #[inline(always)]
    pub fn get_write_pos(&self) -> usize {
        self.write_pos
    }

    #[inline(always)]
    pub fn next(&mut self) -> Option<&u8> {
        self.read_pos += 1;
        self.data.get(self.read_pos as usize)
    }

    #[inline(always)]
    pub fn next_mut(&mut self) -> Option<&mut u8> {
        self.read_pos += 1;
        self.data.get_mut(self.read_pos as usize)
    }

    pub fn skip_byte(&mut self, ch: u8) -> bool {
        match self.data.get((self.read_pos as usize) + 1) {
            Some(x) if *x == ch => {
                self.read_pos += 1;
                true
            },
            _ => false,
        }
    }

    pub fn skip_bytes(&mut self, chs: &[u8]) -> bool {
        let next_pos = (self.read_pos as usize) + 1;
        match self.data.get(next_pos..next_pos+chs.len()) {
            Some(x) if x == chs => {
                self.read_pos += chs.len() as isize;
                true
            },
            _ => false,
        }
    }

    pub fn rewind(&mut self, read_pos: usize) {
        self.read_pos = self.read_pos - std::cmp::min(self.read_pos, read_pos as isize);
    }
}

