use std::{borrow::{Borrow}, cell::Cell};

pub struct MessageStream<'x> {
    data: &'x [u8],
    read_pos: isize,
    write_pos: usize,
    write_start: usize,
    test: Cell<usize>
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &[u8]) -> MessageStream {
        MessageStream {
            data,
            read_pos: -1,
            write_pos: 0,
            write_start: 0,
            test: Cell::new(0)
        }
    }


    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&[u8]> {
        self.test.set(1);
        self.data.get(from..to)
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
        //self.data[self.write_pos] = ch;
        self.write_pos += 1;
    }

    #[inline(always)]
    pub fn write_slice(&mut self, chars: &[u8]) {
        //self.data[self.write_pos..(self.write_pos+chars.len())].copy_from_slice(chars);
        self.write_pos += chars.len();        
    }

    #[inline(always)]
    pub fn write_skip(&mut self) {
        self.write_pos += 1;
    }

    #[inline(always)]
    pub fn end_write(&self) -> Option<&[u8]> {
        self.data.get(self.write_start..self.write_pos)
    }

    #[inline(always)]
    pub fn get_written_bytes(&self) -> isize {
        self.write_pos as isize - self.write_start as isize
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
        None
        //self.data.get_mut(self.read_pos as usize)
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

    /*pub fn skip_spaces(&mut self) -> Option<&u8> {
        loop {
            self.read_pos += 1;
            let ch = self.data.get(self.read_pos as usize)?;
            if !ch.is_ascii_whitespace() {
                return Some(ch);
            }            
        }
    }*/

    pub fn rewind(&mut self, read_pos: usize) {
        self.read_pos = self.read_pos - std::cmp::min(self.read_pos, read_pos as isize);
    }
}

