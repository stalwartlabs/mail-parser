use std::borrow::Cow;

pub struct MessageStream<'x> {
    data: &'x mut [u8],
    read_pos: isize,
    write_pos: usize
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &mut [u8]) -> MessageStream {
        MessageStream {
            data: data,
            read_pos: -1,
            write_pos: 0
        }
    }

    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&[u8]> {
        self.data.get(from..to)
    }

    pub fn get_str_lossy(&'x self, from: usize, to: usize) -> Option<Cow<'x, str>> {
        match self.data.get(from..to) {
            Some(bytes) => Some(String::from_utf8_lossy(bytes)),
            None => None,
        }
    }

    pub fn get_str(&'x self, from: usize, to: usize) -> Option<&str> {
        match self.data.get(from..to) {
            Some(bytes) => Some(unsafe { std::str::from_utf8_unchecked(bytes) }),
            None => None,
        }
    }

    pub fn begin_write(&mut self) -> usize {
        if self.read_pos > 0 {
            self.write_pos = self.read_pos as usize;
        }
        self.write_pos
    }

    pub fn write(&mut self, ch: u8) -> () {
        self.data[self.write_pos] = ch;
        self.write_pos = self.write_pos + 1;
    }

    pub fn write_slice(&mut self, chars: &[u8]) -> () {
        self.data[self.write_pos..(self.write_pos+chars.len())].copy_from_slice(chars);
        self.write_pos = self.write_pos + chars.len();        
    }

    #[inline(always)]
    pub fn get_read_pos(&self) -> isize {
        self.read_pos
    }

    pub fn get_write_pos(&self) -> usize {
        self.write_pos
    }

    #[inline(always)]
    pub fn get_lower_char(&mut self) -> Option<u8> {
        self.read_pos = self.read_pos + 1;
        match self.data.get_mut(self.read_pos as usize) {
            Some(ch) => {
                if *ch >= 65 && *ch <= 90 {
                    *ch = *ch + 32;
                }
                Some(*ch)
            },
            None => None
        }
    }

    pub fn rewind(&mut self, read_pos: usize) -> () {
        self.read_pos = self.read_pos - std::cmp::min(self.read_pos, read_pos as isize);
    }
}

impl<'x> Iterator for MessageStream<'x> {
    type Item = u8;

    #[inline(always)]
    fn next(&mut self) -> Option<u8> {
        self.read_pos = self.read_pos + 1;
        match self.data.get(self.read_pos as usize) {
            Some(ch) => Some(*ch),
            None => None
        }
    }
}
