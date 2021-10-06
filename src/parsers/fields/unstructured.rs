use std::borrow::{Borrow, Cow};

use crate::parsers::{message_stream::MessageStream, rfc2047::Rfc2047Parser};
enum UnstructState {
    Lf,
    Space,
    Char
}

pub fn parse_unstructured<'x>(stream: &'x mut MessageStream) -> Option<Cow<'x, str>> {
    let mut token_start: isize = -1;
    let mut token_end: isize = -1;
    let mut state = UnstructState::Space;
    //let mut tokens = Vec::new();

    while let Some(ch) = stream.next_mut() {
        match ch {
            b'\n' => {
                if let UnstructState::Lf = state {
                    stream.rewind(1);
                    break;
                }
                state = UnstructState::Lf;
                token_start = -1;
            },
            b' ' | b'\t' | b'\r' => {
                state = UnstructState::Space;
            },
            _ => {
                if *ch > 126 {
                    *ch = b'!';
                }

                match state {
                    UnstructState::Lf => {
                        stream.rewind(1);
                        break;                        
                    },
                    UnstructState::Space => {
                        if token_start == -1 {
                            //token_start = stream.get_read_pos();
                        }
                        /*if *ch == b'=' && stream.skip_byte(b'?') {
                            tokens.push(stream.end_write());
                            if let Some(_) = Rfc2047Parser::parse(stream) {
                                tokens.push(stream.end_write());
                                token_start = -1;
                            } else {

                            }
                        }*/

                    },
                    UnstructState::Char => todo!(),
                }

            }
        }

    }

    None
}

