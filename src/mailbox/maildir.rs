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

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Maildir folder iterator
pub struct FolderIterator {
    inbox: Option<MessageIterator>,
    it: fs::ReadDir,
}

/// Maildir message iterator
pub struct MessageIterator {
    name: Option<String>,
    cur_it: fs::ReadDir,
    new_it: fs::ReadDir,
}

/// Maildir message contents and metadata
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Message {
    internal_date: u64,
    flags: Vec<Flag>,
    contents: Vec<u8>,
}

/// Flags of Maildir message
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum Flag {
    Passed,
    Replied,
    Seen,
    Trashed,
    Draft,
    Flagged,
}

impl FolderIterator {
    /// Creates a new Maildir folder iterator
    pub fn new(path: impl Into<PathBuf>) -> io::Result<FolderIterator> {
        let path = path.into();

        Ok(FolderIterator {
            it: fs::read_dir(&path)?,
            inbox: MessageIterator::new_(&path, None)?.into(),
        })
    }
}

impl MessageIterator {
    /// Creates a new Maildir message iterator
    pub fn new(path: impl Into<PathBuf>) -> io::Result<MessageIterator> {
        MessageIterator::new_(&path.into(), None)
    }

    fn new_(path: &Path, name: Option<String>) -> io::Result<MessageIterator> {
        let mut cur_path = path.to_path_buf();
        cur_path.push("cur");
        if !cur_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Invalid Maildir format, 'cur' directory not found.",
            ));
        }
        let mut new_path = path.to_path_buf();
        new_path.push("new");
        if !new_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Invalid Maildir format, 'new' directory not found.",
            ));
        }

        Ok(MessageIterator {
            name,
            cur_it: fs::read_dir(cur_path)?,
            new_it: fs::read_dir(new_path)?,
        })
    }

    /// Returns the mailbox name of None for 'INBOX'.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl Iterator for FolderIterator {
    type Item = io::Result<MessageIterator>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inbox) = self.inbox.take() {
            return Some(Ok(inbox));
        }

        loop {
            let entry = match self.it.next() {
                Some(Ok(entry)) => entry,
                Some(Err(err)) => return Some(Err(err)),
                None => return None,
            };

            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_prefix('.'))
                {
                    match MessageIterator::new_(&path, Some(name.to_string())) {
                        Ok(folder) => return Some(Ok(folder)),
                        Err(err) => {
                            if err.kind() != io::ErrorKind::NotFound {
                                return Some(Err(err));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Iterator for MessageIterator {
    type Item = io::Result<Message>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.cur_it.next().or_else(|| self.new_it.next()) {
                Some(Ok(entry)) => entry,
                Some(Err(err)) => return Some(Err(err)),
                None => return None,
            };
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    if !name.starts_with('.') {
                        let internal_date =
                            match fs::metadata(&path).and_then(|m| m.created()).and_then(|d| {
                                d.duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .map_err(|e| {
                                        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
                                    })
                            }) {
                                Ok(metadata) => metadata,
                                Err(err) => return Some(Err(err)),
                            };
                        let contents = match fs::read(&path) {
                            Ok(contents) => contents,
                            Err(err) => return Some(Err(err)),
                        };
                        let mut flags = Vec::new();
                        if let Some((_, part)) = name.rsplit_once("2,") {
                            for &ch in part.as_bytes() {
                                match ch {
                                    b'P' => flags.push(Flag::Passed),
                                    b'R' => flags.push(Flag::Replied),
                                    b'S' => flags.push(Flag::Seen),
                                    b'T' => flags.push(Flag::Trashed),
                                    b'D' => flags.push(Flag::Draft),
                                    b'F' => flags.push(Flag::Flagged),
                                    _ => {
                                        if !ch.is_ascii_alphanumeric() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        return Some(Ok(Message {
                            contents,
                            internal_date,
                            flags,
                        }));
                    }
                }
            }
        }
    }
}

impl Message {
    pub fn new(internal_date: u64, flags: Vec<Flag>, contents: Vec<u8>) -> Message {
        Message {
            internal_date,
            flags,
            contents,
        }
    }

    /// Returns the message creation date in seconds since UNIX epoch
    pub fn internal_date(&self) -> u64 {
        self.internal_date
    }

    /// Returns the message flags
    pub fn flags(&self) -> &[Flag] {
        &self.flags
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
    use std::path::PathBuf;

    use crate::mailbox::maildir::{Flag, Message};

    use super::FolderIterator;

    #[test]
    fn parse_maildir() {
        let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_dir.push("tests");
        test_dir.push("maildir");

        let mut messages = Vec::new();
        let expected_messages = vec![
            (
                "INBOX".to_string(),
                Message {
                    internal_date: 1660133582,
                    flags: vec![Flag::Seen, Flag::Trashed],
                    contents: vec![97, 10],
                },
            ),
            (
                "INBOX".to_string(),
                Message {
                    internal_date: 1660133743,
                    flags: vec![Flag::Seen],
                    contents: vec![98, 10],
                },
            ),
            (
                "My Folder".to_string(),
                Message {
                    internal_date: 1660133871,
                    flags: vec![Flag::Trashed, Flag::Draft, Flag::Replied],
                    contents: vec![99, 10],
                },
            ),
            (
                "My Folder".to_string(),
                Message {
                    internal_date: 1660133903,
                    flags: vec![],
                    contents: vec![100, 10],
                },
            ),
            (
                "My Folder.Nested Folder".to_string(),
                Message {
                    internal_date: 1660133935,
                    flags: vec![Flag::Flagged, Flag::Passed],
                    contents: vec![101, 10],
                },
            ),
            (
                "My Folder.Nested Folder".to_string(),
                Message {
                    internal_date: 1660133987,
                    flags: vec![Flag::Replied, Flag::Draft, Flag::Flagged],
                    contents: vec![102, 10],
                },
            ),
        ];

        for folder in FolderIterator::new(test_dir).unwrap() {
            let folder = folder.unwrap();
            let name = folder.name().unwrap_or("INBOX").to_string();

            for message in folder {
                messages.push((name.clone(), message.unwrap()));
            }
        }

        messages.sort_unstable();
        assert_eq!(messages, expected_messages);
    }
}
