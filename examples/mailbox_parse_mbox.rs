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

use mail_parser::{mailbox::mbox::MessageIterator, MessageParser};

fn main() {
    // Reads an MBox mailbox from stdin and prints each message as JSON.
    for raw_message in MessageIterator::new(std::io::stdin()) {
        let raw_message = raw_message.unwrap();
        let message = MessageParser::default()
            .parse(raw_message.contents())
            .unwrap();

        println!("{}", serde_json::to_string(&message).unwrap());
    }
}
