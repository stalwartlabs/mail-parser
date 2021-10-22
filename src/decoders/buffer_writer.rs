use std::cell::UnsafeCell;

pub struct BufferWriter<'x> {
    buf: UnsafeCell<&'x mut [u8]>,
    head: UnsafeCell<usize>,
    tail: UnsafeCell<usize>,
}

impl<'x> BufferWriter<'x> {
    pub fn new(buf: &'x mut [u8]) -> BufferWriter<'x> {
        BufferWriter {
            buf: buf.into(),
            head: 0.into(),
            tail: 0.into(),
        }
    }

    pub fn alloc_buffer(capacity: usize) -> Box<[u8]> {
        vec![0u8; capacity].into_boxed_slice()
    }

    pub fn get_buf_mut(&self) -> Option<&mut [u8]> {
        unsafe { (*self.buf.get()).get_mut(*self.tail.get()..) }
    }

    pub fn advance_tail(&self, pos: usize) {
        unsafe {
            let tail = &mut *self.tail.get();
            let new_tail = *tail + pos;

            if new_tail <= (*self.buf.get()).len() {
                *tail = new_tail;
            }
        }
    }

    pub fn reset_tail(&self) {
        unsafe {
            *self.tail.get() = *self.head.get();
        }
    }

    pub fn get_bytes(&self) -> Option<&'x [u8]> {
        unsafe {
            let head = &mut *self.head.get();
            let tail = &mut *self.tail.get();

            if *tail > *head {
                let result = (*self.buf.get()).get_unchecked(*head..*tail);
                *head = *tail;
                Some(result)
            } else {
                None
            }
        }
    }

    pub fn get_string(&self) -> Option<&'x str> {
        unsafe { std::str::from_utf8_unchecked(self.get_bytes()?).into() }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { *self.head.get() == *self.tail.get() }
    }

    fn write_byte(&self, byte: &u8) -> bool {
        unsafe {
            let tail = &mut *self.tail.get();
            let buf = &mut *self.buf.get();

            if *tail < buf.len() {
                *buf.get_unchecked_mut(*tail) = *byte;
                *tail += 1;
                true
            } else {
                debug_assert!(false, "Buffer full, tail {}, len {}", *tail, buf.len());
                false
            }
        }
    }

    fn write_bytes(&self, bytes: &[u8]) -> bool {
        unsafe {
            let tail = &mut *self.tail.get();
            let new_tail = *tail + bytes.len();
            let buf = &mut *self.buf.get();

            if new_tail <= buf.len() {
                buf.get_unchecked_mut(*tail..new_tail)
                    .copy_from_slice(bytes);
                *tail = new_tail;
                true
            } else {
                debug_assert!(
                    false,
                    "Buffer full, tail {}, new tail {}, len {}",
                    *tail,
                    new_tail,
                    buf.len()
                );
                false
            }
        }
    }
}
