use std::borrow::Cow;

use crate::parsers::{
    encoded_word::parse_encoded_word,
    header::{HeaderValue, NamedValue},
    message_stream::MessageStream,
};

#[derive(PartialEq, Clone, Copy, Debug)]
enum AddressState {
    Address,
    Name,
    Quote,
    Comment,
}

pub struct AddressParser<'x> {
    token_start: usize,
    token_end: usize,

    is_token_safe: bool,
    is_token_email: bool,
    is_token_start: bool,
    is_escaped: bool,

    name_tokens: Vec<Cow<'x, str>>,
    mail_tokens: Vec<Cow<'x, str>>,
    comment_tokens: Vec<Cow<'x, str>>,

    state: AddressState,
    state_stack: Vec<AddressState>,

    addresses: Vec<HeaderValue<'x>>,
    group_name: Option<Cow<'x, str>>,
    group_comment: Option<Cow<'x, str>>,
    result: Vec<HeaderValue<'x>>,
}

pub fn add_token<'x>(
    parser: &mut AddressParser<'x>,
    stream: &'x MessageStream,
    add_trail_space: bool,
) {
    if parser.token_start > 0 {
        let token = stream
            .get_string(
                parser.token_start - 1,
                parser.token_end,
                parser.is_token_safe,
            )
            .unwrap();
        let mut add_space = false;
        let list = match parser.state {
            AddressState::Address => &mut parser.mail_tokens,
            AddressState::Name => {
                if parser.is_token_email {
                    &mut parser.mail_tokens
                } else {
                    add_space = true;
                    &mut parser.name_tokens
                }
            }
            AddressState::Quote => &mut parser.name_tokens,
            AddressState::Comment => {
                add_space = true;
                &mut parser.comment_tokens
            }
        };

        if add_space && !list.is_empty() {
            list.push(" ".into());
        }

        list.push(token);

        if add_trail_space {
            list.push(" ".into());
        }

        parser.token_start = 0;
        parser.is_token_safe = true;
        parser.is_token_email = false;
        parser.is_token_start = true;
        parser.is_escaped = false;
    }
}

fn concat_tokens<'x>(tokens: &mut Vec<Cow<'x, str>>) -> Cow<'x, str> {
    if tokens.len() == 1 {
        tokens.pop().unwrap()
    } else {
        let result = tokens.concat();
        tokens.clear();
        result.into()
    }
}

pub fn add_address(parser: &mut AddressParser) {
    let has_mail = !parser.mail_tokens.is_empty();
    let has_name = !parser.name_tokens.is_empty();
    let has_comment = !parser.comment_tokens.is_empty();

    parser
        .addresses
        .push(if has_mail && has_name && has_comment {
            NamedValue::new(
                concat_tokens(&mut parser.name_tokens),
                concat_tokens(&mut parser.comment_tokens).into(),
                HeaderValue::String(concat_tokens(&mut parser.mail_tokens)),
            )
        } else if has_name && has_mail {
            NamedValue::new(
                concat_tokens(&mut parser.name_tokens),
                None,
                HeaderValue::String(concat_tokens(&mut parser.mail_tokens)),
            )
        } else if has_mail && has_comment {
            NamedValue::new(
                concat_tokens(&mut parser.comment_tokens),
                None,
                HeaderValue::String(concat_tokens(&mut parser.mail_tokens)),
            )
        } else if has_mail {
            HeaderValue::String(concat_tokens(&mut parser.mail_tokens))
        } else if has_name && has_comment {
            NamedValue::new(
                concat_tokens(&mut parser.name_tokens),
                concat_tokens(&mut parser.comment_tokens).into(),
                HeaderValue::Empty,
            )
        } else if has_name {
            NamedValue::new(
                concat_tokens(&mut parser.name_tokens),
                None,
                HeaderValue::Empty,
            )
        } else if has_comment {
            NamedValue::new(
                concat_tokens(&mut parser.comment_tokens),
                None,
                HeaderValue::Empty,
            )
        } else {
            return;
        });
}

pub fn add_group_details(parser: &mut AddressParser) {
    if !parser.name_tokens.is_empty() {
        parser.group_name = concat_tokens(&mut parser.name_tokens).into();
    }

    if !parser.comment_tokens.is_empty() {
        parser.group_comment = concat_tokens(&mut parser.comment_tokens).into();
    }

    if !parser.mail_tokens.is_empty() {
        if parser.group_name.is_none() {
            parser.group_name = concat_tokens(&mut parser.mail_tokens).into();
        } else {
            parser.group_name = Some(
                (parser.group_name.as_ref().unwrap().as_ref().to_owned()
                    + " "
                    + concat_tokens(&mut parser.mail_tokens).as_ref())
                .into(),
            );
        }
    }
}

pub fn add_group(parser: &mut AddressParser) {
    let has_name = parser.group_name.is_some();
    let has_addresses = !parser.addresses.is_empty();

    parser.result.push(if has_name && has_addresses {
        NamedValue::new(
            parser.group_name.take().unwrap(),
            parser.group_comment.take(),
            HeaderValue::Array(std::mem::take(&mut parser.addresses)),
        )
    } else if has_addresses {
        HeaderValue::Array(std::mem::take(&mut parser.addresses))
    } else if has_name {
        NamedValue::new(parser.group_name.take().unwrap(), None, HeaderValue::Empty)
    } else {
        return;
    });
}

pub fn parse_address<'x>(stream: &'x MessageStream) -> HeaderValue<'x> {
    let mut parser = AddressParser {
        token_start: 0,
        token_end: 0,

        is_token_safe: true,
        is_token_email: false,
        is_token_start: true,
        is_escaped: false,

        name_tokens: Vec::with_capacity(3),
        mail_tokens: Vec::with_capacity(3),
        comment_tokens: Vec::with_capacity(3),

        state: AddressState::Name,
        state_stack: Vec::with_capacity(5),

        addresses: Vec::new(),
        group_name: None,
        group_comment: None,
        result: Vec::new(),
    };

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => {
                add_token(&mut parser, stream, false);
                match stream.peek() {
                    Some(b' ' | b'\t') => {
                        stream.advance(1);
                        if !parser.is_token_start {
                            parser.is_token_start = true;
                        }
                        continue;
                    }
                    _ => break,
                }
            }
            b'\\' if parser.state != AddressState::Name => {
                if parser.token_start > 0 {
                    if parser.state == AddressState::Quote {
                        parser.token_end = stream.get_pos() - 1;
                    }
                    add_token(&mut parser, stream, false);
                }
                parser.is_escaped = true;
                continue;
            }
            b',' if parser.state == AddressState::Name => {
                add_token(&mut parser, stream, false);
                add_address(&mut parser);
                continue;
            }
            b'<' if parser.state == AddressState::Name => {
                add_token(&mut parser, stream, false);
                parser.state_stack.push(AddressState::Name);
                parser.state = AddressState::Address;
                continue;
            }
            b'>' if parser.state == AddressState::Address => {
                add_token(&mut parser, stream, false);
                parser.state = parser.state_stack.pop().unwrap();
                continue;
            }
            b'"' if !parser.is_escaped => match parser.state {
                AddressState::Name => {
                    parser.state_stack.push(AddressState::Name);
                    parser.state = AddressState::Quote;
                    add_token(&mut parser, stream, false);
                    continue;
                }
                AddressState::Quote => {
                    add_token(&mut parser, stream, false);
                    parser.state = parser.state_stack.pop().unwrap();
                    continue;
                }
                _ => (),
            },
            b'@' if parser.state == AddressState::Name => {
                parser.is_token_email = true;
            }
            b'=' if parser.is_token_start && !parser.is_escaped => {
                if let Some(token) = parse_encoded_word(stream) {
                    let add_space = parser.state != AddressState::Quote; // Make borrow-checker happy
                    add_token(&mut parser, stream, add_space);
                    (if parser.state != AddressState::Comment {
                        &mut parser.name_tokens
                    } else {
                        &mut parser.comment_tokens
                    })
                    .push(token.into());
                    continue;
                }
            }
            b' ' | b'\t' => {
                if !parser.is_token_start {
                    parser.is_token_start = true;
                }
                if parser.is_escaped {
                    parser.is_escaped = false;
                }
                if parser.state == AddressState::Quote {
                    if parser.token_start == 0 {
                        parser.token_start = stream.get_pos();
                        parser.token_end = parser.token_start;
                    } else {
                        parser.token_end = stream.get_pos();
                    }
                }
                continue;
            }
            b'\r' => continue,
            b'(' if parser.state != AddressState::Quote && !parser.is_escaped => {
                parser.state_stack.push(parser.state);
                if parser.state != AddressState::Comment {
                    add_token(&mut parser, stream, false);
                    parser.state = AddressState::Comment;
                    continue;
                }
            }
            b')' if parser.state == AddressState::Comment && !parser.is_escaped => {
                let new_state = parser.state_stack.pop().unwrap();
                if parser.state != new_state {
                    add_token(&mut parser, stream, false);
                    parser.state = new_state;
                    continue;
                }
            }
            b':' if parser.state == AddressState::Name && !parser.is_escaped => {
                add_group(&mut parser);
                add_token(&mut parser, stream, false);
                add_group_details(&mut parser);
                continue;
            }
            b';' if parser.state == AddressState::Name => {
                add_token(&mut parser, stream, false);
                add_address(&mut parser);
                add_group(&mut parser);
                continue;
            }
            0..=0x7f => (),
            _ => {
                if parser.is_token_safe {
                    parser.is_token_safe = false;
                }
            }
        }

        if parser.is_escaped {
            parser.is_escaped = false;
        }

        if parser.is_token_start {
            parser.is_token_start = false;
        }

        if parser.token_start == 0 {
            parser.token_start = stream.get_pos();
            parser.token_end = parser.token_start;
        } else {
            parser.token_end = stream.get_pos();
        }
    }

    add_address(&mut parser);

    if parser.group_name.is_some() || !parser.result.is_empty() {
        add_group(&mut parser);
        if parser.result.len() > 1 {
            HeaderValue::Array(parser.result)
        } else {
            parser.result.pop().unwrap()
        }
    } else if !parser.addresses.is_empty() {
        if parser.addresses.len() > 1 {
            HeaderValue::Array(parser.addresses)
        } else {
            parser.addresses.pop().unwrap()
        }
    } else {
        HeaderValue::Empty
    }
}

mod tests {
    use crate::parsers::{fields::address::parse_address, message_stream::MessageStream};

    use super::*;

    #[test]
    fn parse_addresses() {
        let inputs = [
            (
                concat!("John Doe <jdoe@machine.example>\n"),
                NamedValue::new(
                    "John Doe".into(),
                    None,
                    HeaderValue::String("jdoe@machine.example".into()),
                ),
            ),
            (
                concat!(" Mary Smith <mary@example.net>\n"),
                NamedValue::new(
                    "Mary Smith".into(),
                    None,
                    HeaderValue::String("mary@example.net".into()),
                ),
            ),
            (
                concat!("\"Joe Q. Public\" <john.q.public@example.com>\n"),
                NamedValue::new(
                    "Joe Q. Public".into(),
                    None,
                    HeaderValue::String("john.q.public@example.com".into()),
                ),
            ),
            (
                concat!("Mary Smith <mary@x.test>, jdoe@example.org, Who? <one@y.test>\n"),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "Mary Smith".into(),
                        None,
                        HeaderValue::String("mary@x.test".into()),
                    ),
                    HeaderValue::String("jdoe@example.org".into()),
                    NamedValue::new(
                        "Who?".into(),
                        None,
                        HeaderValue::String("one@y.test".into()),
                    ),
                ]),
            ),
            (
                concat!("<boss@nil.test>, \"Giant; \\\"Big\\\" Box\" <sysservices@example.net>\n"),
                HeaderValue::Array(vec![
                    HeaderValue::String("boss@nil.test".into()),
                    NamedValue::new(
                        "Giant; \"Big\" Box".into(),
                        None,
                        HeaderValue::String("sysservices@example.net".into()),
                    ),
                ]),
            ),
            (
                concat!("A Group:Ed Jones <c@a.test>,joe@where.test,John <jdoe@one.test>;\n"),
                NamedValue::new(
                    "A Group".into(),
                    None,
                    HeaderValue::Array(vec![
                        NamedValue::new(
                            "Ed Jones".into(),
                            None,
                            HeaderValue::String("c@a.test".into()),
                        ),
                        HeaderValue::String("joe@where.test".into()),
                        NamedValue::new(
                            "John".into(),
                            None,
                            HeaderValue::String("jdoe@one.test".into()),
                        ),
                    ]),
                ),
            ),
            (
                concat!("Undisclosed recipients:;\n"),
                NamedValue::new("Undisclosed recipients".into(), None, HeaderValue::Empty),
            ),
            (
                concat!("\"Mary Smith: Personal Account\" <smith@home.example >\n"),
                NamedValue::new(
                    "Mary Smith: Personal Account".into(),
                    None,
                    HeaderValue::String("smith@home.example".into()),
                ),
            ),
            (
                concat!("Pete(A nice \\) chap) <pete(his account)@silly.test(his host)>\n"),
                NamedValue::new(
                    "Pete".into(),
                    Some("A nice ) chap his account his host".into()),
                    HeaderValue::String("pete@silly.test".into()),
                ),
            ),
            (
                concat!(
                    "Pete(A nice \n \\\n ) chap) <pete(his\n account)@silly\n .test(his host)>\n"
                ),
                NamedValue::new(
                    "Pete".into(),
                    Some("A nice ) chap his account his host".into()),
                    HeaderValue::String("pete@silly.test".into()),
                ),
            ),
            (
                concat!(
                    "A Group(Some people)\n        :Chris Jones <c@(Chris's host.)public.exa",
                    "mple>,\n            joe@example.org,\n     John <jdoe@one.test> (my dear",
                    " friend); (the end of the group)\n"
                ),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "A Group".into(),
                        Some("Some people".into()),
                        HeaderValue::Array(vec![
                            NamedValue::new(
                                "Chris Jones".into(),
                                Some("Chris's host.".into()),
                                HeaderValue::String("c@public.example".into()),
                            ),
                            HeaderValue::String("joe@example.org".into()),
                            NamedValue::new(
                                "John".into(),
                                Some("my dear friend".into()),
                                HeaderValue::String("jdoe@one.test".into()),
                            ),
                        ]),
                    ),
                    HeaderValue::Array(vec![NamedValue::new(
                        "the end of the group".into(),
                        None,
                        HeaderValue::Empty,
                    )]),
                ]),
            ),
            (
                concat!("(Empty list)(start)Hidden recipients  :(nobody(that I know))  ;\n"),
                NamedValue::new(
                    "Hidden recipients".into(),
                    Some("Empty list start".into()),
                    HeaderValue::Array(vec![NamedValue::new(
                        "nobody(that I know)".into(),
                        None,
                        HeaderValue::Empty,
                    )]),
                ),
            ),
            (
                concat!("Joe Q. Public <john.q.public@example.com>\n"),
                NamedValue::new(
                    "Joe Q. Public".into(),
                    None,
                    HeaderValue::String("john.q.public@example.com".into()),
                ),
            ),
            (
                concat!("Mary Smith <@node.test:mary@example.net>, , jdoe@test  . example\n"),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "Mary Smith".into(),
                        None,
                        HeaderValue::String("@node.test:mary@example.net".into()),
                    ),
                    HeaderValue::String("jdoe@test  . example".into()),
                ]),
            ),
            (
                concat!("John Doe <jdoe@machine(comment).  example>\n"),
                NamedValue::new(
                    "John Doe".into(),
                    Some("comment".into()),
                    HeaderValue::String("jdoe@machine.  example".into()),
                ),
            ),
            (
                concat!("Mary Smith\n    \n\t<mary@example.net>\n"),
                NamedValue::new(
                    "Mary Smith".into(),
                    None,
                    HeaderValue::String("mary@example.net".into()),
                ),
            ),
            (
                concat!("=?US-ASCII*EN?Q?Keith_Moore?= <moore@cs.utk.edu>\n"),
                NamedValue::new(
                    "Keith Moore".into(),
                    None,
                    HeaderValue::String("moore@cs.utk.edu".into()),
                ),
            ),
            (
                concat!("John =?US-ASCII*EN?Q?Doe?= <moore@cs.utk.edu>\n"),
                NamedValue::new(
                    "John Doe".into(),
                    None,
                    HeaderValue::String("moore@cs.utk.edu".into()),
                ),
            ),
            (
                concat!("=?ISO-8859-1?Q?Keld_J=F8rn_Simonsen?= <keld@dkuug.dk>\n"),
                NamedValue::new(
                    "Keld Jørn Simonsen".into(),
                    None,
                    HeaderValue::String("keld@dkuug.dk".into()),
                ),
            ),
            (
                concat!("=?ISO-8859-1?Q?Andr=E9?= Pirard <PIRARD@vm1.ulg.ac.be>\n"),
                NamedValue::new(
                    "André Pirard".into(),
                    None,
                    HeaderValue::String("PIRARD@vm1.ulg.ac.be".into()),
                ),
            ),
            (
                concat!("=?ISO-8859-1?Q?Olle_J=E4rnefors?= <ojarnef@admin.kth.se>\n"),
                NamedValue::new(
                    "Olle Järnefors".into(),
                    None,
                    HeaderValue::String("ojarnef@admin.kth.se".into()),
                ),
            ),
            (
                concat!("ietf-822@dimacs.rutgers.edu, ojarnef@admin.kth.se\n"),
                HeaderValue::Array(vec![
                    HeaderValue::String("ietf-822@dimacs.rutgers.edu".into()),
                    HeaderValue::String("ojarnef@admin.kth.se".into()),
                ]),
            ),
            (
                concat!(
                    "Nathaniel Borenstein <nsb@thumper.bellcore.com>\n    (=?iso-8859-8?b?7e",
                    "Xs+SDv4SDp7Oj08A==?=)\n"
                ),
                NamedValue::new(
                    "Nathaniel Borenstein".into(),
                    Some("םולש ןב ילטפנ".into()),
                    HeaderValue::String("nsb@thumper.bellcore.com".into()),
                ),
            ),
            (
                concat!(
                    "Greg Vaudreuil <gvaudre@NRI.Reston.VA.US>, Ned Freed\n      <ned@innoso",
                    "ft.com>, Keith Moore <moore@cs.utk.edu>\n"
                ),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "Greg Vaudreuil".into(),
                        None,
                        HeaderValue::String("gvaudre@NRI.Reston.VA.US".into()),
                    ),
                    NamedValue::new(
                        "Ned Freed".into(),
                        None,
                        HeaderValue::String("ned@innosoft.com".into()),
                    ),
                    NamedValue::new(
                        "Keith Moore".into(),
                        None,
                        HeaderValue::String("moore@cs.utk.edu".into()),
                    ),
                ]),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?= <test@test.com>\n"),
                NamedValue::new(
                    "a".into(),
                    None,
                    HeaderValue::String("test@test.com".into()),
                ),
            ),
            (
                concat!("\"=?ISO-8859-1?Q?a?= b\" <test@test.com>\n"),
                NamedValue::new(
                    "a b".into(),
                    None,
                    HeaderValue::String("test@test.com".into()),
                ),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?b?= <test@test.com>\n"),
                NamedValue::new(
                    "ab".into(),
                    None,
                    HeaderValue::String("test@test.com".into()),
                ),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?=\n   =?ISO-8859-1?Q?b?= <test@test.com>\n"),
                NamedValue::new(
                    "ab".into(),
                    None,
                    HeaderValue::String("test@test.com".into()),
                ),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?= \"=?ISO-8859-2?Q?_b?=\" <test@test.com>\n"),
                NamedValue::new(
                    "a b".into(),
                    None,
                    HeaderValue::String("test@test.com".into()),
                ),
            ),
            (
                concat!(" <test@test.com>\n"),
                HeaderValue::String("test@test.com".into()),
            ),
            (
                concat!("test@test.com\ninvalid@address.com\n"),
                HeaderValue::String("test@test.com".into()),
            ),
            (
                concat!(
                    "\"=?ISO-8859-1?Q =?ISO-8859-1?Q?a?= \\\" =?ISO-8859-1?Q?b?=\" <last@addres",
                    "s.com>\n\nbody@content.com"
                ),
                NamedValue::new(
                    "=?ISO-8859-1?Q a \" b".into(),
                    None,
                    HeaderValue::String("last@address.com".into()),
                ),
            ),
            (
                concat!("=? <name@domain.com>\n"),
                NamedValue::new(
                    "=?".into(),
                    None,
                    HeaderValue::String("name@domain.com".into()),
                ),
            ),
            (
                concat!(
                    "\"  James Smythe\" <james@example.com>, Friends:\n  jane@example.com, =?U",
                    "TF-8?Q?John_Sm=C3=AEth?=\n   <john@example.com>;\n"
                ),
                HeaderValue::Array(vec![
                    HeaderValue::Array(vec![NamedValue::new(
                        "  James Smythe".into(),
                        None,
                        HeaderValue::String("james@example.com".into()),
                    )]),
                    NamedValue::new(
                        "Friends".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("jane@example.com".into()),
                            NamedValue::new(
                                "John Smîth".into(),
                                None,
                                HeaderValue::String("john@example.com".into()),
                            ),
                        ]),
                    ),
                ]),
            ),
            (
                concat!(
                    "List 1: addr1@test.com, addr2@test.com; List 2: addr3@test.com, addr4@",
                    "test.com; addr5@test.com, addr6@test.com\n"
                ),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "List 1".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("addr1@test.com".into()),
                            HeaderValue::String("addr2@test.com".into()),
                        ]),
                    ),
                    NamedValue::new(
                        "List 2".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("addr3@test.com".into()),
                            HeaderValue::String("addr4@test.com".into()),
                        ]),
                    ),
                    HeaderValue::Array(vec![
                        HeaderValue::String("addr5@test.com".into()),
                        HeaderValue::String("addr6@test.com".into()),
                    ]),
                ]),
            ),
            (
                concat!(
                    "\"List 1\": addr1@test.com, addr2@test.com; \"List 2\": addr3@test.com, ad",
                    "dr4@test.com; addr5@test.com, addr6@test.com\n"
                ),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "List 1".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("addr1@test.com".into()),
                            HeaderValue::String("addr2@test.com".into()),
                        ]),
                    ),
                    NamedValue::new(
                        "List 2".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("addr3@test.com".into()),
                            HeaderValue::String("addr4@test.com".into()),
                        ]),
                    ),
                    HeaderValue::Array(vec![
                        HeaderValue::String("addr5@test.com".into()),
                        HeaderValue::String("addr6@test.com".into()),
                    ]),
                ]),
            ),
            (
                concat!(
                    "\"=?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=\": addr1@test.com, addr2@",
                    "test.com; =?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=: addr3@test.com",
                    ", addr4@test.com; addr5@test.com, addr6@test.com\n"
                ),
                HeaderValue::Array(vec![
                    NamedValue::new(
                        "Thís ís válíd ÚTF8".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("addr1@test.com".into()),
                            HeaderValue::String("addr2@test.com".into()),
                        ]),
                    ),
                    NamedValue::new(
                        "Thís ís válíd ÚTF8".into(),
                        None,
                        HeaderValue::Array(vec![
                            HeaderValue::String("addr3@test.com".into()),
                            HeaderValue::String("addr4@test.com".into()),
                        ]),
                    ),
                    HeaderValue::Array(vec![
                        HeaderValue::String("addr5@test.com".into()),
                        HeaderValue::String("addr6@test.com".into()),
                    ]),
                ]),
            ),
        ];

        for input in inputs {
            let stream = &MessageStream::new(input.0.as_bytes());
            let result = parse_address(stream);

            /*println!(
                "(concat!({}), {}),",
                format!(
                    "{:?}",
                    input
                        .0
                        .chars()
                        .collect::<Vec<char>>()
                        .chunks(70)
                        .map(|c| c.iter().collect::<String>())
                        .collect::<Vec<String>>()
                )
                .replace("[\"", "\"")
                .replace("\"]", "\""),
                format!("{:?}", result)
                    .replace("NamedValue(NamedValue {", "NamedValue::new(")
                    .replace("Array([", "HeaderValue::Array(vec![")
                    .replace("String(", "HeaderValue::String(")
                    .replace("})", ")")
                    .replace(", subname:", ".into(),")
                    .replace("name: ", "")
                    .replace("\", value:", "\".into(),")
                    .replace(", value:", ", ")
                    .replace("\")", "\".into())")
                    .replace(" Empty", " HeaderValue::Empty")
            );*/

            /*println!(
                "{} ->\n{}\n{}\n",
                input.0.escape_debug(),
                str,
                "-".repeat(50)
            );*/

            assert_eq!(result, input.1, "Failed for '{}'", input.0.escape_debug());
        }
    }
}
