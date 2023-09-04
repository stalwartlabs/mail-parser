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
