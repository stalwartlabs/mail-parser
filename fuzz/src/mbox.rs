use mail_parser::mailbox::mbox::{Message, MessageIterator};
use std::io::{self, BufRead, BufReader, Read};

const SEPARATOR: &[u8] = b"From ";
const QUOTE: u8 = b'>';
const CAPACITIES: [usize; 7] = [1, 2, 3, 7, 16, 64, 4_096];
const CHUNK_SIZES: usize = 4;
const MAX_CHUNK: usize = 9;
const CHUNKED_CAPACITY: usize = 16;

#[derive(Debug, PartialEq, Eq)]
struct Expected {
    from: String,
    contents: Vec<u8>,
}

fn unquoted(line: &[u8]) -> &[u8] {
    let quotes = line.iter().take_while(|&&byte| byte == QUOTE).count();
    match (line.get(quotes..), line.get(1..)) {
        (Some(rest), Some(unquoted)) if quotes > 0 && rest.starts_with(SEPARATOR) => unquoted,
        _ => line,
    }
}

fn sender(line: &[u8]) -> String {
    line.strip_prefix(SEPARATOR)
        .and_then(|rest| std::str::from_utf8(rest).ok())
        .and_then(|rest| rest.split_once(' '))
        .map_or_else(String::new, |(from, _)| from.trim().to_string())
}

fn reference(mbox: &[u8]) -> Vec<Expected> {
    let mut messages = Vec::new();
    let mut current: Option<Expected> = None;
    for line in mbox.split_inclusive(|&byte| byte == b'\n') {
        if line.starts_with(SEPARATOR) {
            messages.extend(current.replace(Expected {
                from: sender(line),
                contents: Vec::new(),
            }));
        } else if let Some(message) = &mut current {
            message.contents.extend_from_slice(unquoted(line));
        }
    }
    messages.extend(current);
    messages
}

fn read_all(reader: impl BufRead) -> Vec<Message> {
    MessageIterator::new(reader)
        .map(|message| message.expect("an in-memory mbox is readable"))
        .collect()
}

struct Chunked<'x> {
    data: &'x [u8],
    sizes: [usize; CHUNK_SIZES],
    next: usize,
}

impl Read for Chunked<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = self.sizes.get(self.next).copied().unwrap_or(1);
        self.next = (self.next + 1) % CHUNK_SIZES;
        let len = size.min(buf.len()).min(self.data.len());
        let (head, tail) = self.data.split_at(len);
        if let Some(out) = buf.get_mut(..len) {
            out.copy_from_slice(head);
        }
        self.data = tail;
        Ok(len)
    }
}

pub fn check(data: &[u8]) {
    let whole = read_all(data);
    let expected = reference(data);
    assert_eq!(whole.len(), expected.len(), "message count");
    for (message, expected) in whole.iter().zip(&expected) {
        assert_eq!(message.from(), expected.from, "separator sender");
        assert_eq!(message.contents(), expected.contents, "contents");
        let _ = message.internal_date();
    }
    for capacity in CAPACITIES {
        assert_eq!(
            read_all(BufReader::with_capacity(capacity, data)),
            whole,
            "capacity {capacity}"
        );
    }
    let mut sizes = [1; CHUNK_SIZES];
    for (size, &byte) in sizes.iter_mut().zip(data.iter().rev()) {
        *size = 1 + usize::from(byte) % MAX_CHUNK;
    }
    let chunked = Chunked {
        data,
        sizes,
        next: 0,
    };
    assert_eq!(
        read_all(BufReader::with_capacity(CHUNKED_CAPACITY, chunked)),
        whole,
        "chunked reader {sizes:?}"
    );
    for message in whole {
        let contents = message.unwrap_contents();
        let _ = mail_parser::MessageParser::new().parse(&contents);
    }
}
