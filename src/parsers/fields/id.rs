use std::borrow::Cow;

use crate::parsers::message_stream::MessageStream;

#[derive(PartialEq, Debug)]
pub enum IdField<'x> {
    One(Cow<'x, str>),
    Many(Vec<Cow<'x, str>>),
    Empty
}

pub fn parse_id<'x>(stream: &'x MessageStream) -> IdField<'x> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut is_token_safe = true;
    let mut is_id_part = false;
    let mut ids = Vec::new();

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => {
                match stream.peek() {
                    Some(b' ' | b'\t') => {
                        stream.advance(1);
                        continue;
                    },
                    _ => {
                        return match ids.len() {
                            1 => IdField::One(ids.pop().unwrap()),
                            0 => IdField::Empty,
                            _ => IdField::Many(ids),
                        };
                    },
                }
            }
            b'<' => {
                is_id_part = true;
                continue;
            }
            b'>' => {
                is_id_part = false;
                if token_start > 0 {
                    let bytes = stream.get_bytes(token_start - 1, token_end).unwrap();

                    ids.push(if is_token_safe {
                        Cow::from(unsafe { std::str::from_utf8_unchecked(bytes) })
                    } else {
                        is_token_safe = true;
                        String::from_utf8_lossy(bytes)
                    });
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
    IdField::Empty
}

mod tests {
    use crate::parsers::{fields::id::{IdField, parse_id}, message_stream::MessageStream};


    #[test]
    fn parse_message_ids() {
        let inputs = [
            ("<1234@local.machine.example>\n".to_string(), vec!["1234@local.machine.example"]),
            ("<1234@local.machine.example> <3456@example.net>\n".to_string(), vec!["1234@local.machine.example", "3456@example.net"]),
            ("<1234@local.machine.example>\n <3456@example.net> \n".to_string(), vec!["1234@local.machine.example", "3456@example.net"]),
            ("<1234@local.machine.example>\n\n <3456@example.net>\n".to_string(), vec!["1234@local.machine.example"]),
            ("              <testabcd.1234@silly.test>  \n".to_string(), vec!["testabcd.1234@silly.test"]),
            ("<5678.21-Nov-1997@example.com>\n".to_string(), vec!["5678.21-Nov-1997@example.com"]),
            ("<1234   @   local(blah)  .machine .example>\n".to_string(), vec!["1234   @   local(blah)  .machine .example"]),
        ];

        for input in inputs {
            match parse_id(&MessageStream::new(input.0.as_bytes())) {
                IdField::One(val) => assert_eq!(val, *input.1.first().unwrap()),
                IdField::Many(val) => assert_eq!(val, input.1),
                IdField::Empty => panic!("Failed to parse '{}'", input.0),
            }
        }
    }
}

