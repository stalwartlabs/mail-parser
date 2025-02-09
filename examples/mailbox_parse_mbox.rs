/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{mailbox::mbox::MessageIterator, MessageParser};

fn main() {
    // Reads an MBox mailbox from stdin and prints each message as JSON.
    for raw_message in MessageIterator::new(std::io::stdin().lock()) {
        let raw_message = raw_message.unwrap();
        let message = MessageParser::default()
            .parse(raw_message.contents())
            .unwrap();

        println!("{}", serde_json::to_string(&message).unwrap());
    }
}
