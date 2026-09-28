/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{MessageBuffers, MessageParser, mailbox::mbox::MessageIterator};
use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
};

const SAMPLE: &[u8] = b"From ann@example.com Sat Jan  3 01:05:34 1996\n\
From: Ann <ann@example.com>\n\
Subject: First message\n\
\n\
>From the mbox point of view, this line was quoted.\n\
\n\
From bob@example.com Sun Jan  4 10:00:00 1996\n\
From: Bob <bob@example.com>\n\
Subject: Second message\n\
\n\
Hello.\n";

fn main() -> io::Result<()> {
    match env::args_os().nth(1) {
        None => list(SAMPLE),
        Some(path) if path == "-" => list(io::stdin().lock()),
        Some(path) => list(BufReader::new(File::open(path)?)),
    }
}

fn list(mbox: impl BufRead) -> io::Result<()> {
    let parser = MessageParser::new();
    let mut buffers = MessageBuffers::new();
    for entry in MessageIterator::new(mbox) {
        let entry = entry?;
        match parser.parse_with(entry.contents(), &mut buffers) {
            Some(message) => {
                println!(
                    "{} from {}: {:?}, {} parts, preview {:?}",
                    entry.internal_date(),
                    entry.from(),
                    message.subject().unwrap_or_default(),
                    message.parts().len(),
                    message.body_preview(60).unwrap_or_default()
                );
                buffers = message.into_buffers();
            }
            None => println!(
                "{} from {}: not a message",
                entry.internal_date(),
                entry.from()
            ),
        }
    }
    Ok(())
}
