/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::io::{BufRead, BufReader, Read};

/// Parses an Mbox mailbox from a `Read` stream, returning each message as a
/// `Vec<u8>`.
/// supports >From  quoting as defined in the [QMail mbox specification](http://qmail.org/qmail-manual-html/man5/mbox.html).
pub struct MessageIterator<T: Read> {
    reader: BufReader<T>,
    found_from: bool,
}

#[derive(Debug)]
pub struct ParseError {}

impl<T> MessageIterator<T>
where
    T: Read,
{
    pub fn new(reader: T) -> MessageIterator<T> {
        MessageIterator {
            reader: BufReader::new(reader),
            found_from: false,
        }
    }
}

impl<T> Iterator for MessageIterator<T>
where
    T: Read,
{
    type Item = Result<Vec<u8>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut message = Vec::with_capacity(1024);
        let mut message_line = Vec::with_capacity(80);

        loop {
            match self.reader.read_until(b'\n', &mut message_line) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }
                }
                Err(_) => {
                    return Some(Err(ParseError {}));
                }
            }

            let is_from = message_line
                .get(..5)
                .map(|line| line == b"From ")
                .unwrap_or(false);

            if self.found_from {
                if !is_from {
                    if message_line[0] != b'>' {
                        message.append(&mut message_line);
                    } else if message_line
                        .iter()
                        .skip_while(|&&ch| ch == b'>')
                        .take(5)
                        .copied()
                        .collect::<Vec<u8>>()
                        == b"From "
                    {
                        message.extend_from_slice(&message_line[1..]);
                        message_line.clear();
                    } else {
                        message.append(&mut message_line);
                    }
                } else {
                    break;
                }
            } else {
                if is_from {
                    self.found_from = true;
                }
                message_line.clear();
            }
        }

        if !message.is_empty() {
            Some(Ok(message))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MessageIterator;

    #[test]
    fn parse_mbox() {
        let message = br#"From Mon, 15 Jan 2018 15:30:00 +0100
Message 1

From Mon, 15 Jan 2018 15:30:00 +0100
Message 2

From Mon, 15 Jan 2018 15:30:00 +0100
Message 3
>From hello
>>From world
>>>From test

From Mon, 15 Jan 2018 15:30:00 +0100
Message 4
> From
>F
"#;

        let mut parser = MessageIterator::new(&message[..]);

        assert_eq!(parser.next().unwrap().unwrap(), b"Message 1\n\n");
        assert_eq!(parser.next().unwrap().unwrap(), b"Message 2\n\n");
        assert_eq!(
            parser.next().unwrap().unwrap(),
            b"Message 3\nFrom hello\n>From world\n>>From test\n\n"
        );
        assert_eq!(parser.next().unwrap().unwrap(), b"Message 4\n> From\n>F\n");
        assert!(parser.next().is_none());
    }
}
