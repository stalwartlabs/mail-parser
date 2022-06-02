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

use crate::{parsers::message::MessageStream, HeaderValue};

pub fn parse_id<'x>(stream: &mut MessageStream<'x>) -> HeaderValue<'x> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut token_invalid_start: usize = 0; // Handle broken clients
    let mut token_invalid_end: usize = 0; // Handle broken clients
    let mut is_id_part = false;
    let mut ids = Vec::new();

    let mut iter = stream.data[stream.pos..].iter();

    while let Some(ch) = iter.next() {
        stream.pos += 1;
        match ch {
            b'\n' => match stream.data.get(stream.pos) {
                Some(b' ' | b'\t') => {
                    iter.next();
                    stream.pos += 1;
                    continue;
                }
                _ => {
                    return match ids.len() {
                        1 => HeaderValue::Text(ids.pop().unwrap()),
                        0 => {
                            if token_invalid_start > 0 {
                                HeaderValue::Text(String::from_utf8_lossy(
                                    &stream.data[token_invalid_start - 1..token_invalid_end],
                                ))
                            } else {
                                HeaderValue::Empty
                            }
                        }
                        _ => HeaderValue::TextList(ids),
                    };
                }
            },
            b'<' => {
                is_id_part = true;
                continue;
            }
            b'>' => {
                is_id_part = false;
                if token_start > 0 {
                    ids.push(String::from_utf8_lossy(
                        &stream.data[token_start - 1..token_end],
                    ));
                    token_start = 0;
                } else {
                    continue;
                }
            }
            b' ' | b'\t' | b'\r' => continue,
            _ => {}
        }
        if is_id_part {
            if token_start == 0 {
                token_start = stream.pos;
            }
            token_end = stream.pos;
        } else {
            if token_invalid_start == 0 {
                token_invalid_start = stream.pos;
            }
            token_invalid_end = stream.pos;
        }
    }

    HeaderValue::Empty
}

#[cfg(test)]
mod tests {
    use crate::parsers::fields::id::parse_id;
    use crate::parsers::message::MessageStream;
    use crate::HeaderValue;

    #[test]
    fn parse_message_ids() {
        let inputs = [
            (
                "<1234@local.machine.example>\n",
                vec!["1234@local.machine.example"],
            ),
            (
                "<1234@local.machine.example> <3456@example.net>\n",
                vec!["1234@local.machine.example", "3456@example.net"],
            ),
            (
                "<1234@local.machine.example>\n <3456@example.net> \n",
                vec!["1234@local.machine.example", "3456@example.net"],
            ),
            (
                "<1234@local.machine.example>\n\n <3456@example.net>\n",
                vec!["1234@local.machine.example"],
            ),
            (
                "              <testabcd.1234@silly.test>  \n",
                vec!["testabcd.1234@silly.test"],
            ),
            (
                "<5678.21-Nov-1997@example.com>\n",
                vec!["5678.21-Nov-1997@example.com"],
            ),
            (
                "<1234   @   local(blah)  .machine .example>\n",
                vec!["1234   @   local(blah)  .machine .example"],
            ),
            ("<>\n", vec![""]),
            // Malformed Ids should be parsed anyway
            (
                "malformed@id.machine.example\n",
                vec!["malformed@id.machine.example"],
            ),
            (
                "   malformed2@id.machine.example \t  \n",
                vec!["malformed2@id.machine.example"],
            ),
            ("   m \n", vec!["m"]),
        ];

        for input in inputs {
            let str = input.0.to_string();
            match parse_id(&mut MessageStream::new(str.as_bytes())) {
                HeaderValue::TextList(ids) => {
                    assert_eq!(ids, input.1, "Failed to parse '{:?}'", input.0);
                }
                HeaderValue::Text(id) => {
                    assert!(input.1.len() == 1, "Failed to parse '{:?}'", input.0);
                    assert_eq!(id, input.1[0], "Failed to parse '{:?}'", input.0);
                }
                HeaderValue::Empty if input.1[0].is_empty() => {}
                result => panic!("Unexpected result: {:?}", result),
            }
        }
    }
}
