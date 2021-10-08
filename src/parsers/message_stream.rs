use std::cell::Cell;

pub struct MessageStream<'x> {
    data: &'x [u8],
    pos: Cell<usize>
}

impl<'x> MessageStream<'x> {
    pub fn new(data: &'x [u8]) -> MessageStream<'x> {
        MessageStream {
            data,
            pos: Cell::new(0)
        }
    }

    #[inline(always)]
    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&[u8]> {
        self.data.get(from..to)
    }

    #[inline(always)]
    pub fn set_pos(&self, pos: usize) {
        self.pos.set(pos)
    }

    #[inline(always)]
    pub fn set_pos_2(&mut self, pos: usize) {
        self.pos.set(pos)
    }


    #[inline(always)]
    pub fn get_pos(&self) -> usize {
        self.pos.get()
    }

    #[inline(always)]
    pub fn next(&self) -> Option<&u8> {
        let pos = self.pos.get();
        self.pos.set(pos + 1);
        self.data.get(pos)
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<&u8> {
        self.data.get(self.pos.get())
    }

    #[inline(always)]
    pub fn advance(&self, pos: usize) {
        self.pos.set(self.pos.get() + pos);
    }    

    pub fn skip_byte(&self, ch: &u8) -> bool {
        let pos = self.pos.get();
        match self.data.get(pos) {
            Some(byte) if byte == ch => {
                self.pos.set(pos + 1);
                true
            },
            _ => false
        }
    }

    pub fn skip_bytes(&self, chs: &[u8]) -> bool {
        let from = self.pos.get();
        let to = from + chs.len();

        match self.data.get(from..to) {
            Some(bytes) if bytes == chs => {
                self.pos.set(to);
                true
            },
            _ => false
        }        
    }

    pub fn rewind(&self, r: usize) {
        let pos = self.pos.get() - r;
        self.pos.set(pos);
    }
}