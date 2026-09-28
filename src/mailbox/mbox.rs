/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Mbox files.

use crate::{DateTime, MONTH};
use memchr::{memchr, memmem::Finder, memrchr};
use std::io::{self, BufRead};

const SEPARATOR: &[u8] = b"From ";
const QUOTE: u8 = b'>';
const CONTENTS_CAPACITY: usize = 1024;

/// Iterates the messages of an mbox read from a buffered reader.
///
/// A message starts at each line beginning with `From ` (the separator line,
/// which gives [`Message::from`] and [`Message::internal_date`]) and runs to
/// the next one. Lines before the first separator are skipped. Lines quoted
/// as `>From `, `>>From `, ... lose one `>`, as described in the
/// [qmail mbox specification](http://qmail.org/qmail-manual-html/man5/mbox.html).
/// Separators are found with `memchr`, and the contents are copied from the
/// reader's buffer in bulk.
///
/// ```
/// use mail_parser::mailbox::mbox::MessageIterator;
///
/// let mbox = b"From ann@example.com Sat Jan  3 01:05:34 1996\n\
/// Subject: hello\n\n>From the start\n";
/// let message = MessageIterator::new(&mbox[..])
///     .next()
///     .expect("one message")
///     .expect("readable mbox");
/// assert_eq!(message.from(), "ann@example.com");
/// assert_eq!(message.internal_date(), 820_631_134);
/// assert_eq!(message.contents(), b"Subject: hello\n\nFrom the start\n");
/// ```
#[derive(Debug)]
pub struct MessageIterator<T> {
    reader: T,
    message: Option<Message>,
    line: Vec<u8>,
    complete: usize,
    separator: Finder<'static>,
}

/// A message of an mbox: its contents and the metadata of its separator
/// line.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Message {
    internal_date: u64,
    from: String,
    contents: Vec<u8>,
}

impl<T> MessageIterator<T>
where
    T: BufRead,
{
    /// Iterates over the messages of the mbox read from `reader`.
    pub fn new(reader: T) -> MessageIterator<T> {
        MessageIterator {
            reader,
            message: None,
            line: Vec::new(),
            complete: 0,
            separator: Finder::new(SEPARATOR),
        }
    }
}

impl<T> Iterator for MessageIterator<T>
where
    T: BufRead,
{
    type Item = io::Result<Message>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.line.is_empty() {
                let buf = match self.reader.fill_buf() {
                    Ok(buf) => buf,
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => return Some(Err(err)),
                };
                if buf.is_empty() {
                    return self.message.take().map(Ok);
                }
                if self.complete == 0 || self.complete > buf.len() {
                    self.complete = memrchr(b'\n', buf).map_or(0, |last| last + 1);
                }
                if let Some(lines) = buf.get(..self.complete).filter(|lines| !lines.is_empty()) {
                    let (consumed, finished) = split(&self.separator, &mut self.message, lines);
                    self.reader.consume(consumed);
                    self.complete -= consumed;
                    if finished.is_some() {
                        return finished.map(Ok);
                    }
                    continue;
                }
            }
            if let Err(err) = self.reader.read_until(b'\n', &mut self.line) {
                return Some(Err(err));
            }
            let (_, finished) = split(&self.separator, &mut self.message, &self.line);
            self.line.clear();
            if finished.is_some() {
                return finished.map(Ok);
            }
        }
    }
}

fn split(
    separator: &Finder<'_>,
    message: &mut Option<Message>,
    lines: &[u8],
) -> (usize, Option<Message>) {
    let mut copied = 0;
    let mut offset = 0;
    while let Some(hit) = lines
        .get(offset..)
        .and_then(|rest| separator.find(rest))
        .map(|found| offset + found)
    {
        offset = hit + SEPARATOR.len();
        let before = lines.get(..hit).unwrap_or_default();
        let quotes = before
            .iter()
            .rev()
            .take_while(|&&byte| byte == QUOTE)
            .count();
        let Some((head, _)) = before.split_at_checked(before.len() - quotes) else {
            continue;
        };
        if !matches!(head.last(), None | Some(b'\n')) {
            continue;
        }
        let line_start = head.len();
        append(message, lines.get(copied..line_start));
        if quotes > 0 {
            copied = line_start + 1;
            continue;
        }
        let line_end = lines
            .get(hit..)
            .and_then(|rest| memchr(b'\n', rest))
            .map_or(lines.len(), |end| hit + end + 1);
        let finished = message.replace(Message::new(
            lines.get(line_start..line_end).unwrap_or_default(),
        ));
        if finished.is_some() {
            return (line_end, finished);
        }
        copied = line_end;
        offset = line_end;
    }
    append(message, lines.get(copied..));
    (lines.len(), None)
}

fn append(message: &mut Option<Message>, bytes: Option<&[u8]>) {
    if let (Some(message), Some(bytes)) = (message, bytes) {
        message.contents.extend_from_slice(bytes);
    }
}

fn internal_date(date: &str) -> u64 {
    let mut dt = DateTime {
        year: u16::MAX,
        month: u8::MAX,
        day: u8::MAX,
        hour: u8::MAX,
        minute: u8::MAX,
        second: u8::MAX,
        tz_before_gmt: false,
        tz_hour: 0,
        tz_minute: 0,
    };
    for (pos, part) in date.split_whitespace().enumerate() {
        match pos {
            1 => {
                dt.month = MONTH
                    .iter()
                    .zip(1..)
                    .find(|(name, _)| part.eq_ignore_ascii_case(name))
                    .map_or(u8::MAX, |(_, month)| month);
            }
            2 => dt.day = part.parse().unwrap_or(u8::MAX),
            3 => {
                let fields = [&mut dt.hour, &mut dt.minute, &mut dt.second];
                for (field, value) in fields.into_iter().zip(part.split(':')) {
                    *field = value.parse().unwrap_or(u8::MAX);
                }
            }
            4 => dt.year = part.parse().unwrap_or(u16::MAX),
            _ => (),
        }
    }
    if dt.is_valid() {
        dt.to_timestamp() as u64
    } else {
        0
    }
}

impl Message {
    fn new(line: &[u8]) -> Self {
        let (internal_date, from) = line
            .strip_prefix(SEPARATOR)
            .and_then(|rest| std::str::from_utf8(rest).ok())
            .and_then(|rest| rest.split_once(' '))
            .map_or((0, String::new()), |(from, date)| {
                (internal_date(date), from.trim().to_string())
            });
        Message {
            internal_date,
            from,
            contents: Vec::with_capacity(CONTENTS_CAPACITY),
        }
    }

    /// The date of the separator line, in seconds since the UNIX epoch; 0
    /// when it is missing or does not parse.
    pub fn internal_date(&self) -> u64 {
        self.internal_date
    }

    /// The sender address of the separator line; empty when missing.
    pub fn from(&self) -> &str {
        &self.from
    }

    /// The message, ready for [`crate::MessageParser::parse`].
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    /// Takes the message contents.
    pub fn unwrap_contents(self) -> Vec<u8> {
        self.contents
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, MessageIterator};
    use crate::scan::tests::Rng;
    use std::io::{self, BufRead, BufReader, Read};

    const MBOX: &[u8] = br#"From god@heaven.af.mil Sat Jan  3 01:05:34 1996
Message 1

From cras@irccrew.org  Tue Jul 23 19:39:23 2002
Message 2

From test@test.com Tue Aug  6 13:34:34 2002
Message 3
>From hello
>>From world
>>>From test

From other@domain.com Mon Jan 15  15:30:00  2018
Message 4
> From
>F
"#;

    fn expected() -> Vec<Message> {
        vec![
            Message {
                internal_date: 820631134,
                from: "god@heaven.af.mil".to_string(),
                contents: b"Message 1\n\n".to_vec(),
            },
            Message {
                internal_date: 1027453163,
                from: "cras@irccrew.org".to_string(),
                contents: b"Message 2\n\n".to_vec(),
            },
            Message {
                internal_date: 1028640874,
                from: "test@test.com".to_string(),
                contents: b"Message 3\nFrom hello\n>From world\n>>From test\n\n".to_vec(),
            },
            Message {
                internal_date: 1516030200,
                from: "other@domain.com".to_string(),
                contents: b"Message 4\n> From\n>F\n".to_vec(),
            },
        ]
    }

    fn read_all(reader: impl BufRead) -> Vec<Message> {
        MessageIterator::new(reader)
            .map(|message| message.expect("message is readable"))
            .collect()
    }

    fn unquoted(line: &[u8]) -> &[u8] {
        let quotes = line.iter().take_while(|&&byte| byte == b'>').count();
        match (line.get(quotes..), line.get(1..)) {
            (Some(rest), Some(unquoted)) if quotes > 0 && rest.starts_with(b"From ") => unquoted,
            _ => line,
        }
    }

    fn reference(mbox: &[u8]) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut current: Option<Message> = None;
        for line in mbox.split_inclusive(|&byte| byte == b'\n') {
            if line.starts_with(b"From ") {
                messages.extend(current.replace(Message::new(line)));
            } else if let Some(message) = &mut current {
                message.contents.extend_from_slice(unquoted(line));
            }
        }
        messages.extend(current);
        messages
    }

    struct Chunked<'x> {
        data: &'x [u8],
        sizes: Vec<usize>,
        next: usize,
    }

    impl Read for Chunked<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let size = self.sizes.get(self.next).copied().unwrap_or(1).max(1);
            self.next = (self.next + 1) % self.sizes.len().max(1);
            let len = size.min(buf.len()).min(self.data.len());
            let (head, tail) = self.data.split_at(len);
            buf.get_mut(..len)
                .expect("length fits")
                .copy_from_slice(head);
            self.data = tail;
            Ok(len)
        }
    }

    #[test]
    fn parse_mbox() {
        assert_eq!(read_all(MBOX), expected());
        assert_eq!(reference(MBOX), expected());
    }

    #[test]
    fn buffer_boundaries() {
        for capacity in 1..=64 {
            assert_eq!(
                read_all(BufReader::with_capacity(capacity, MBOX)),
                expected(),
                "capacity {capacity}"
            );
        }
    }

    #[test]
    fn same_as_line_reader() {
        const PIECES: [&[u8]; 14] = [
            b"From ",
            b">",
            b">>",
            b"From a@b Sat Jan  3 01:05:34 1996",
            b"\n",
            b"\r\n",
            b"text",
            b" ",
            b"From",
            b"F",
            b"rom ",
            b">From x",
            b"x@y",
            b"From \xff Tue Jul 23 19:39:23 2002",
        ];
        let mut rng = Rng(0x6d62_6f78_0000_0001);
        for round in 0..4_000 {
            let mbox: Vec<u8> = (0..rng.below(40))
                .flat_map(|_| {
                    PIECES
                        .get(rng.below(PIECES.len()))
                        .copied()
                        .unwrap_or_default()
                })
                .copied()
                .collect();
            let expected = reference(&mbox);
            assert_eq!(read_all(&mbox[..]), expected, "round {round}: {mbox:?}");
            let capacity = 1 + rng.below(12);
            assert_eq!(
                read_all(BufReader::with_capacity(capacity, &mbox[..])),
                expected,
                "round {round} capacity {capacity}: {mbox:?}"
            );
            let sizes = (0..4).map(|_| 1 + rng.below(9)).collect();
            let chunked = Chunked {
                data: &mbox,
                sizes,
                next: 0,
            };
            assert_eq!(
                read_all(BufReader::with_capacity(16, chunked)),
                expected,
                "round {round} chunked: {mbox:?}"
            );
        }
    }

    #[test]
    fn long_lines_and_missing_final_newline() {
        let long = "x".repeat(10_000);
        let mbox = format!(
            "{long}\nFrom a@b Sat Jan  3 01:05:34 1996\n{long}\n>From {long}\nFrom c@d {long}"
        );
        let expected = reference(mbox.as_bytes());
        assert_eq!(expected.len(), 2);
        assert_eq!(read_all(mbox.as_bytes()), expected);
        assert_eq!(
            read_all(BufReader::with_capacity(64, mbox.as_bytes())),
            expected
        );
    }
}
