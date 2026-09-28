/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{MessageBuffers, MessageParser, mailbox::maildir::FolderIterator};
use std::{env, io, path::PathBuf};

fn main() -> io::Result<()> {
    let maildir = env::args_os().nth(1).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/maildir"),
        PathBuf::from,
    );
    let parser = MessageParser::new();
    let mut buffers = MessageBuffers::new();
    for folder in FolderIterator::new(maildir, Some("."))? {
        let folder = folder?;
        println!("Folder {}", folder.name().unwrap_or("INBOX"));
        for entry in folder {
            let entry = entry?;
            print!(
                "  {}, flags {:?}, {} bytes: ",
                entry.path().display(),
                entry.flags(),
                entry.contents().len()
            );
            match parser.parse_with(entry.contents(), &mut buffers) {
                Some(message) => {
                    println!("{:?}", message.subject().unwrap_or_default());
                    buffers = message.into_buffers();
                }
                None => println!("not a message"),
            }
        }
    }
    Ok(())
}
