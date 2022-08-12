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

use crate::DateTime;
use std::io::{BufRead, BufReader, Read};

/// Parses an Mbox mailbox from a `Read` stream, returning each message as a
/// `Vec<u8>`.
/// supports >From  quoting as defined in the [QMail mbox specification](http://qmail.org/qmail-manual-html/man5/mbox.html).
pub struct MessageIterator<T: Read> {
    reader: BufReader<T>,
    message: Option<Message>,
}

/// Mbox message contents and metadata
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Message {
    internal_date: u64,
    from: String,
    contents: Vec<u8>,
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
            message: None,
        }
    }
}

impl<T> Iterator for MessageIterator<T>
where
    T: Read,
{
    type Item = Result<Message, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
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

            if let Some(message) = &mut self.message {
                if !is_from {
                    if message_line[0] != b'>' {
                        message.contents.append(&mut message_line);
                    } else if message_line
                        .iter()
                        .skip_while(|&&ch| ch == b'>')
                        .take(5)
                        .copied()
                        .collect::<Vec<u8>>()
                        == b"From "
                    {
                        message.contents.extend_from_slice(&message_line[1..]);
                        message_line.clear();
                    } else {
                        message.contents.append(&mut message_line);
                    }
                } else {
                    let message = self.message.take().map(Ok);
                    self.message =
                        Message::new(std::str::from_utf8(&message_line).unwrap_or("")).into();
                    return message;
                }
            } else {
                if is_from {
                    self.message =
                        Message::new(std::str::from_utf8(&message_line).unwrap_or("")).into();
                }
                message_line.clear();
            }
        }

        self.message.take().map(Ok)
    }
}

impl Message {
    fn new(hdr: &str) -> Self {
        let (internal_date, from) = if let Some((from, date)) = hdr
            .strip_prefix("From ")
            .and_then(|hdr| hdr.split_once(' '))
        {
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
                        dt.month = if part.eq_ignore_ascii_case("jan") {
                            1
                        } else if part.eq_ignore_ascii_case("feb") {
                            2
                        } else if part.eq_ignore_ascii_case("mar") {
                            3
                        } else if part.eq_ignore_ascii_case("apr") {
                            4
                        } else if part.eq_ignore_ascii_case("may") {
                            5
                        } else if part.eq_ignore_ascii_case("jun") {
                            6
                        } else if part.eq_ignore_ascii_case("jul") {
                            7
                        } else if part.eq_ignore_ascii_case("aug") {
                            8
                        } else if part.eq_ignore_ascii_case("sep") {
                            9
                        } else if part.eq_ignore_ascii_case("oct") {
                            10
                        } else if part.eq_ignore_ascii_case("nov") {
                            11
                        } else if part.eq_ignore_ascii_case("dec") {
                            12
                        } else {
                            u8::MAX
                        };
                    }
                    2 => {
                        dt.day = part.parse().unwrap_or(u8::MAX);
                    }
                    3 => {
                        for (pos, part) in part.split(':').enumerate() {
                            match pos {
                                0 => {
                                    dt.hour = part.parse().unwrap_or(u8::MAX);
                                }
                                1 => {
                                    dt.minute = part.parse().unwrap_or(u8::MAX);
                                }
                                2 => {
                                    dt.second = part.parse().unwrap_or(u8::MAX);
                                }
                                _ => {
                                    break;
                                }
                            }
                        }
                    }
                    4 => {
                        dt.year = part.parse().unwrap_or(u16::MAX);
                    }
                    _ => (),
                }
            }

            (
                if dt.is_valid() {
                    dt.to_timestamp() as u64
                } else {
                    0
                },
                from.trim().to_string(),
            )
        } else {
            (0, "".to_string())
        };

        Self {
            internal_date,
            from,
            contents: Vec::with_capacity(1024),
        }
    }

    /// Returns the message creation date in UTC seconds since UNIX epoch
    pub fn internal_date(&self) -> u64 {
        self.internal_date
    }

    /// Returns the message sender address
    pub fn from(&self) -> &str {
        &self.from
    }

    /// Returns the message contents
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    /// Unwraps the message contents
    pub fn unwrap_contents(self) -> Vec<u8> {
        self.contents
    }
}

#[cfg(test)]
mod tests {
    use crate::mailbox::mbox::Message;

    use super::MessageIterator;

    #[test]
    fn parse_mbox() {
        let message = br#"From god@heaven.af.mil Sat Jan  3 01:05:34 1996
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

        let parser = MessageIterator::new(&message[..]);
        let expected_messages = vec![
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
        ];

        for (message, expected_messages) in parser.zip(expected_messages) {
            assert_eq!(message.unwrap(), expected_messages);
        }
    }
}
