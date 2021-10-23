use std::borrow::Cow;

use crate::parsers::message_stream::MessageStream;

pub fn parse_id<'x>(stream: &MessageStream<'x>) -> Option<Vec<Cow<'x, str>>> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut is_token_safe = true;
    let mut is_id_part = false;
    let mut ids = Vec::new();

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => match stream.peek() {
                Some(b' ' | b'\t') => {
                    stream.advance(1);
                    continue;
                }
                _ => return if !ids.is_empty() { Some(ids) } else { None },
            },
            b'<' => {
                is_id_part = true;
                continue;
            }
            b'>' => {
                is_id_part = false;
                if token_start > 0 {
                    ids.push(
                        stream
                            .get_string(token_start - 1, token_end, is_token_safe)
                            .unwrap(),
                    );
                    is_token_safe = true;
                    token_start = 0;
                } else {
                    continue;
                }
            }
            b' ' | b'\t' | b'\r' => continue,
            0..=0x7f => (),
            _ => {
                if is_token_safe {
                    is_token_safe = false;
                }
            }
        }
        if is_id_part {
            if token_start == 0 {
                token_start = stream.get_pos();
            }
            token_end = stream.get_pos();
        }
    }
    None
}

mod tests {
    use std::borrow::Cow;

    use crate::parsers::{fields::id::parse_id, message_stream::MessageStream};

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
        ];

        for input in inputs {
            let mut str = input.0.to_string();
            assert_eq!(
                input.1,
                parse_id(&MessageStream::new(unsafe { str.as_bytes_mut() })).unwrap(),
                "Failed to parse '{:?}'",
                input.0
            );
        }
    }
}
