/*
 * Copyright Stalwart Labs, Minter Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use mail_parser::{parsers::mbox::MBoxParser, Message};

fn main() {
    // Read an MBox mailbox from stdin and prints each message as YAML.

    for raw_message in MBoxParser::new(std::io::stdin()) {
        let message = Message::parse(&raw_message).unwrap();
        println!("{}", serde_yaml::to_string(&message).unwrap());
    }
}
