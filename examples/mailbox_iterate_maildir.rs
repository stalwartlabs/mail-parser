/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::path::PathBuf;

use mail_parser::mailbox::maildir::FolderIterator;

fn main() {
    // Iterates a Maildir++ structure printing the results to stdout.
    for folder in FolderIterator::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("maildir"),
        ".".into(),
    )
    .unwrap()
    {
        let folder = folder.unwrap();
        println!("------\nMailbox: {:?}", folder.name().unwrap_or("INBOX"));

        for message in folder {
            let message = message.unwrap();
            println!(
                "Message with internal date {}, flags {:?} and content {:?}.",
                message.internal_date(),
                message.flags(),
                String::from_utf8_lossy(message.contents())
            );
        }
    }
}
