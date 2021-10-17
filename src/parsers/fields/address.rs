use std::borrow::Cow;

use crate::{decoders::encoded_word::parse_encoded_word, parsers::message_stream::MessageStream};

#[derive(Debug, PartialEq)]
pub struct Addr<'x> {
    name: Option<Cow<'x, str>>,
    address: Option<Cow<'x, str>>,
}

#[derive(Debug, PartialEq)]
pub struct Group<'x> {
    name: Option<Cow<'x, str>>,
    addresses: Vec<Addr<'x>>,
}

#[derive(Debug, PartialEq)]
pub enum Address<'x> {
    Address(Addr<'x>),
    AddressList(Vec<Addr<'x>>),
    Group(Group<'x>),
    GroupList(Vec<Group<'x>>),
    Collection(Vec<Address<'x>>),
    Empty,
}

impl<'x> Default for Address<'x> {
    fn default() -> Self {
        Address::Empty
    }
}

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

    addresses: Vec<Addr<'x>>,
    group_name: Option<Cow<'x, str>>,
    group_comment: Option<Cow<'x, str>>,
    result: Vec<Group<'x>>,
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
            Addr {
                name: Some(
                    format!(
                        "{} ({})",
                        concat_tokens(&mut parser.name_tokens),
                        concat_tokens(&mut parser.comment_tokens)
                    )
                    .into(),
                ),
                address: concat_tokens(&mut parser.mail_tokens).into(),
            }
        } else if has_name && has_mail {
            Addr {
                name: concat_tokens(&mut parser.name_tokens).into(),
                address: concat_tokens(&mut parser.mail_tokens).into(),
            }
        } else if has_mail && has_comment {
            Addr {
                name: concat_tokens(&mut parser.comment_tokens).into(),
                address: concat_tokens(&mut parser.mail_tokens).into(),
            }
        } else if has_mail {
            Addr {
                name: None,
                address: concat_tokens(&mut parser.mail_tokens).into(),
            }
        } else if has_name && has_comment {
            Addr {
                name: concat_tokens(&mut parser.comment_tokens).into(),
                address: concat_tokens(&mut parser.name_tokens).into(),
            }
        } else if has_name {
            Addr {
                name: concat_tokens(&mut parser.name_tokens).into(),
                address: None,
            }
        } else if has_comment {
            Addr {
                name: concat_tokens(&mut parser.comment_tokens).into(),
                address: None,
            }
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
    let has_comment = parser.group_comment.is_some();
    let has_addresses = !parser.addresses.is_empty();

    parser
        .result
        .push(if has_name && has_addresses && has_comment {
            Group {
                name: Some(
                    format!(
                        "{} ({})",
                        parser.group_name.take().unwrap(),
                        parser.group_comment.take().unwrap()
                    )
                    .into(),
                ),
                addresses: std::mem::take(&mut parser.addresses),
            }
        } else if has_addresses && has_name {
            Group {
                name: parser.group_name.take(),
                addresses: std::mem::take(&mut parser.addresses),
            }
        } else if has_addresses {
            Group {
                name: parser.group_comment.take(),
                addresses: std::mem::take(&mut parser.addresses),
            }
        } else if has_name {
            Group {
                name: parser.group_name.take(),
                addresses: Vec::new(),
            }
        } else {
            return;
        });
}

pub fn parse_address<'x>(stream: &'x MessageStream) -> Address<'x> {
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
            Address::GroupList(parser.result)
        } else {
            Address::Group(parser.result.pop().unwrap())
        }
    } else if !parser.addresses.is_empty() {
        if parser.addresses.len() > 1 {
            Address::AddressList(parser.addresses)
        } else {
            Address::Address(parser.addresses.pop().unwrap())
        }
    } else {
        Address::Empty
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
                Address::Address(Addr {
                    name: Some("John Doe".into()),
                    address: Some("jdoe@machine.example".into()),
                }),
            ),
            (
                concat!(" Mary Smith <mary@example.net>\n"),
                Address::Address(Addr {
                    name: Some("Mary Smith".into()),
                    address: Some("mary@example.net".into()),
                }),
            ),
            (
                concat!("\"Joe Q. Public\" <john.q.public@example.com>\n"),
                Address::Address(Addr {
                    name: Some("Joe Q. Public".into()),
                    address: Some("john.q.public@example.com".into()),
                }),
            ),
            (
                concat!("Mary Smith <mary@x.test>, jdoe@example.org, Who? <one@y.test>\n"),
                Address::AddressList(vec![
                    Addr {
                        name: Some("Mary Smith".into()),
                        address: Some("mary@x.test".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("jdoe@example.org".into()),
                    },
                    Addr {
                        name: Some("Who?".into()),
                        address: Some("one@y.test".into()),
                    },
                ]),
            ),
            (
                concat!("<boss@nil.test>, \"Giant; \\\"Big\\\" Box\" <sysservices@example.net>\n"),
                Address::AddressList(vec![
                    Addr {
                        name: None,
                        address: Some("boss@nil.test".into()),
                    },
                    Addr {
                        name: Some("Giant; \"Big\" Box".into()),
                        address: Some("sysservices@example.net".into()),
                    },
                ]),
            ),
            (
                concat!("A Group:Ed Jones <c@a.test>,joe@where.test,John <jdoe@one.test>;\n"),
                Address::Group(Group {
                    name: Some("A Group".into()),
                    addresses: vec![
                        Addr {
                            name: Some("Ed Jones".into()),
                            address: Some("c@a.test".into()),
                        },
                        Addr {
                            name: None,
                            address: Some("joe@where.test".into()),
                        },
                        Addr {
                            name: Some("John".into()),
                            address: Some("jdoe@one.test".into()),
                        },
                    ],
                }),
            ),
            (
                concat!("Undisclosed recipients:;\n"),
                Address::Group(Group {
                    name: Some("Undisclosed recipients".into()),
                    addresses: vec![],
                }),
            ),
            (
                concat!("\"Mary Smith: Personal Account\" <smith@home.example >\n"),
                Address::Address(Addr {
                    name: Some("Mary Smith: Personal Account".into()),
                    address: Some("smith@home.example".into()),
                }),
            ),
            (
                concat!("Pete(A nice \\) chap) <pete(his account)@silly.test(his host)>\n"),
                Address::Address(Addr {
                    name: Some("Pete (A nice ) chap his account his host)".into()),
                    address: Some("pete@silly.test".into()),
                }),
            ),
            (
                concat!(
                    "Pete(A nice \n \\\n ) chap) <pete(his\n account)@silly\n .test(his host)>\n"
                ),
                Address::Address(Addr {
                    name: Some("Pete (A nice ) chap his account his host)".into()),
                    address: Some("pete@silly.test".into()),
                }),
            ),
            (
                concat!(
                    "A Group(Some people)\n        :Chris Jones <c@(Chris's host.)public.exa",
                    "mple>,\n            joe@example.org,\n     John <jdoe@one.test> (my dear",
                    " friend); (the end of the group)\n"
                ),
                Address::GroupList(vec![
                    Group {
                        name: Some("A Group (Some people)".into()),
                        addresses: vec![
                            Addr {
                                name: Some("Chris Jones (Chris's host.)".into()),
                                address: Some("c@public.example".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("joe@example.org".into()),
                            },
                            Addr {
                                name: Some("John (my dear friend)".into()),
                                address: Some("jdoe@one.test".into()),
                            },
                        ],
                    },
                    Group {
                        name: None,
                        addresses: vec![Addr {
                            name: Some("the end of the group".into()),
                            address: None,
                        }],
                    },
                ]),
            ),
            (
                concat!("(Empty list)(start)Hidden recipients  :(nobody(that I know))  ;\n"),
                Address::Group(Group {
                    name: Some("Hidden recipients (Empty list start)".into()),
                    addresses: vec![Addr {
                        name: Some("nobody(that I know)".into()),
                        address: None,
                    }],
                }),
            ),
            (
                concat!("Joe Q. Public <john.q.public@example.com>\n"),
                Address::Address(Addr {
                    name: Some("Joe Q. Public".into()),
                    address: Some("john.q.public@example.com".into()),
                }),
            ),
            (
                concat!("Mary Smith <@node.test:mary@example.net>, , jdoe@test  . example\n"),
                Address::AddressList(vec![
                    Addr {
                        name: Some("Mary Smith".into()),
                        address: Some("@node.test:mary@example.net".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("jdoe@test  . example".into()),
                    },
                ]),
            ),
            (
                concat!("John Doe <jdoe@machine(comment).  example>\n"),
                Address::Address(Addr {
                    name: Some("John Doe (comment)".into()),
                    address: Some("jdoe@machine.  example".into()),
                }),
            ),
            (
                concat!("Mary Smith\n    \n\t<mary@example.net>\n"),
                Address::Address(Addr {
                    name: Some("Mary Smith".into()),
                    address: Some("mary@example.net".into()),
                }),
            ),
            (
                concat!("=?US-ASCII*EN?Q?Keith_Moore?= <moore@cs.utk.edu>\n"),
                Address::Address(Addr {
                    name: Some("Keith Moore".into()),
                    address: Some("moore@cs.utk.edu".into()),
                }),
            ),
            (
                concat!("John =?US-ASCII*EN?Q?Doe?= <moore@cs.utk.edu>\n"),
                Address::Address(Addr {
                    name: Some("John Doe".into()),
                    address: Some("moore@cs.utk.edu".into()),
                }),
            ),
            (
                concat!("=?ISO-8859-1?Q?Keld_J=F8rn_Simonsen?= <keld@dkuug.dk>\n"),
                Address::Address(Addr {
                    name: Some("Keld Jørn Simonsen".into()),
                    address: Some("keld@dkuug.dk".into()),
                }),
            ),
            (
                concat!("=?ISO-8859-1?Q?Andr=E9?= Pirard <PIRARD@vm1.ulg.ac.be>\n"),
                Address::Address(Addr {
                    name: Some("André Pirard".into()),
                    address: Some("PIRARD@vm1.ulg.ac.be".into()),
                }),
            ),
            (
                concat!("=?ISO-8859-1?Q?Olle_J=E4rnefors?= <ojarnef@admin.kth.se>\n"),
                Address::Address(Addr {
                    name: Some("Olle Järnefors".into()),
                    address: Some("ojarnef@admin.kth.se".into()),
                }),
            ),
            (
                concat!("ietf-822@dimacs.rutgers.edu, ojarnef@admin.kth.se\n"),
                Address::AddressList(vec![
                    Addr {
                        name: None,
                        address: Some("ietf-822@dimacs.rutgers.edu".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("ojarnef@admin.kth.se".into()),
                    },
                ]),
            ),
            (
                concat!(
                    "Nathaniel Borenstein <nsb@thumper.bellcore.com>\n    (=?iso-8859-8?b?7e",
                    "Xs+SDv4SDp7Oj08A==?=)\n"
                ),
                Address::Address(Addr {
                    name: Some("Nathaniel Borenstein (םולש ןב ילטפנ)".into()),
                    address: Some("nsb@thumper.bellcore.com".into()),
                }),
            ),
            (
                concat!(
                    "Greg Vaudreuil <gvaudre@NRI.Reston.VA.US>, Ned Freed\n      <ned@innoso",
                    "ft.com>, Keith Moore <moore@cs.utk.edu>\n"
                ),
                Address::AddressList(vec![
                    Addr {
                        name: Some("Greg Vaudreuil".into()),
                        address: Some("gvaudre@NRI.Reston.VA.US".into()),
                    },
                    Addr {
                        name: Some("Ned Freed".into()),
                        address: Some("ned@innosoft.com".into()),
                    },
                    Addr {
                        name: Some("Keith Moore".into()),
                        address: Some("moore@cs.utk.edu".into()),
                    },
                ]),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?= <test@test.com>\n"),
                Address::Address(Addr {
                    name: Some("a".into()),
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!("\"=?ISO-8859-1?Q?a?= b\" <test@test.com>\n"),
                Address::Address(Addr {
                    name: Some("a b".into()),
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?b?= <test@test.com>\n"),
                Address::Address(Addr {
                    name: Some("ab".into()),
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?=\n   =?ISO-8859-1?Q?b?= <test@test.com>\n"),
                Address::Address(Addr {
                    name: Some("ab".into()),
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!("=?ISO-8859-1?Q?a?= \"=?ISO-8859-2?Q?_b?=\" <test@test.com>\n"),
                Address::Address(Addr {
                    name: Some("a b".into()),
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!(" <test@test.com>\n"),
                Address::Address(Addr {
                    name: None,
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!("test@test.com\ninvalid@address.com\n"),
                Address::Address(Addr {
                    name: None,
                    address: Some("test@test.com".into()),
                }),
            ),
            (
                concat!(
                    "\"=?ISO-8859-1?Q =?ISO-8859-1?Q?a?= \\\" =?ISO-8859-1?Q?b?=\" <last@addres",
                    "s.com>\n\nbody@content.com"
                ),
                Address::Address(Addr {
                    name: Some("=?ISO-8859-1?Q a \" b".into()),
                    address: Some("last@address.com".into()),
                }),
            ),
            (
                concat!("=? <name@domain.com>\n"),
                Address::Address(Addr {
                    name: Some("=?".into()),
                    address: Some("name@domain.com".into()),
                }),
            ),
            (
                concat!(
                    "\"  James Smythe\" <james@example.com>, Friends:\n  jane@example.com, =?U",
                    "TF-8?Q?John_Sm=C3=AEth?=\n   <john@example.com>;\n"
                ),
                Address::GroupList(vec![
                    Group {
                        name: None,
                        addresses: vec![Addr {
                            name: Some("  James Smythe".into()),
                            address: Some("james@example.com".into()),
                        }],
                    },
                    Group {
                        name: Some("Friends".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("jane@example.com".into()),
                            },
                            Addr {
                                name: Some("John Smîth".into()),
                                address: Some("john@example.com".into()),
                            },
                        ],
                    },
                ]),
            ),
            (
                concat!(
                    "List 1: addr1@test.com, addr2@test.com; List 2: addr3@test.com, addr4@",
                    "test.com; addr5@test.com, addr6@test.com\n"
                ),
                Address::GroupList(vec![
                    Group {
                        name: Some("List 1".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr1@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr2@test.com".into()),
                            },
                        ],
                    },
                    Group {
                        name: Some("List 2".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr3@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr4@test.com".into()),
                            },
                        ],
                    },
                    Group {
                        name: None,
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr5@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr6@test.com".into()),
                            },
                        ],
                    },
                ]),
            ),
            (
                concat!(
                    "\"List 1\": addr1@test.com, addr2@test.com; \"List 2\": addr3@test.com, ad",
                    "dr4@test.com; addr5@test.com, addr6@test.com\n"
                ),
                Address::GroupList(vec![
                    Group {
                        name: Some("List 1".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr1@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr2@test.com".into()),
                            },
                        ],
                    },
                    Group {
                        name: Some("List 2".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr3@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr4@test.com".into()),
                            },
                        ],
                    },
                    Group {
                        name: None,
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr5@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr6@test.com".into()),
                            },
                        ],
                    },
                ]),
            ),
            (
                concat!(
                    "\"=?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=\": addr1@test.com, addr2@",
                    "test.com; =?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=: addr3@test.com",
                    ", addr4@test.com; addr5@test.com, addr6@test.com\n"
                ),
                Address::GroupList(vec![
                    Group {
                        name: Some("Thís ís válíd ÚTF8".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr1@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr2@test.com".into()),
                            },
                        ],
                    },
                    Group {
                        name: Some("Thís ís válíd ÚTF8".into()),
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr3@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr4@test.com".into()),
                            },
                        ],
                    },
                    Group {
                        name: None,
                        addresses: vec![
                            Addr {
                                name: None,
                                address: Some("addr5@test.com".into()),
                            },
                            Addr {
                                name: None,
                                address: Some("addr6@test.com".into()),
                            },
                        ],
                    },
                ]),
            ),
            (
                concat!("<http://www.host.com/list/archive/> (Web Archive)\n"),
                Address::Address(Addr {
                    name: Some("Web Archive".into()),
                    address: Some("http://www.host.com/list/archive/".into()),
                }),
            ),
            (
                concat!("<mailto:archive@host.com?subject=index%20list>\n"),
                Address::Address(Addr {
                    name: None,
                    address: Some("mailto:archive@host.com?subject=index%20list".into()),
                }),
            ),
            (
                concat!("<mailto:moderator@host.com> (Postings are Moderated)\n"),
                Address::Address(Addr {
                    name: Some("Postings are Moderated".into()),
                    address: Some("mailto:moderator@host.com".into()),
                }),
            ),
            (
                concat!(
                    "(Use this command to join the list)\n   <mailto:list-manager@host.com?b",
                    "ody=subscribe%20list>\n"
                ),
                Address::Address(Addr {
                    name: Some("Use this command to join the list".into()),
                    address: Some("mailto:list-manager@host.com?body=subscribe%20list".into()),
                }),
            ),
            (
                concat!(
                    "<http://www.host.com/list.cgi?cmd=sub&lst=list>,\n   <mailto:list-manag",
                    "er@host.com?body=subscribe%20list>\n"
                ),
                Address::AddressList(vec![
                    Addr {
                        name: None,
                        address: Some("http://www.host.com/list.cgi?cmd=sub&lst=list".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("mailto:list-manager@host.com?body=subscribe%20list".into()),
                    },
                ]),
            ),
            (
                concat!("NO (posting not allowed on this list)\n"),
                Address::Address(Addr {
                    name: Some("posting not allowed on this list".into()),
                    address: Some("NO".into()),
                }),
            ),
            (
                concat!(
                    "<ftp://ftp.host.com/list.txt> (FTP),\n   <mailto:list@host.com?subject=",
                    "help>\n"
                ),
                Address::AddressList(vec![
                    Addr {
                        name: Some("FTP".into()),
                        address: Some("ftp://ftp.host.com/list.txt".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("mailto:list@host.com?subject=help".into()),
                    },
                ]),
            ),
            (
                concat!("<http://www.host.com/list/>, <mailto:list-info@host.com>\n"),
                Address::AddressList(vec![
                    Addr {
                        name: None,
                        address: Some("http://www.host.com/list/".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("mailto:list-info@host.com".into()),
                    },
                ]),
            ),
            (
                concat!(
                    "(Use this command to get off the list)\n     <mailto:list-manager@host.",
                    "com?body=unsubscribe%20list>\n"
                ),
                Address::Address(Addr {
                    name: Some("Use this command to get off the list".into()),
                    address: Some("mailto:list-manager@host.com?body=unsubscribe%20list".into()),
                }),
            ),
            (
                concat!(
                    "<http://www.host.com/list.cgi?cmd=unsub&lst=list>,\n   <mailto:list-req",
                    "uest@host.com?subject=unsubscribe>\n"
                ),
                Address::AddressList(vec![
                    Addr {
                        name: None,
                        address: Some("http://www.host.com/list.cgi?cmd=unsub&lst=list".into()),
                    },
                    Addr {
                        name: None,
                        address: Some("mailto:list-request@host.com?subject=unsubscribe".into()),
                    },
                ]),
            ),
            (
                concat!("<mailto:listmom@host.com> (Contact Person for Help)\n"),
                Address::Address(Addr {
                    name: Some("Contact Person for Help".into()),
                    address: Some("mailto:listmom@host.com".into()),
                }),
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
                    .replace("GroupList([", "Address::GroupList(vec![")
                    .replace("Address(Addr", "Address::Address(Addr")
                    .replace("AddressList([", "Address::AddressList(vec![")
                    .replace("Group(Group", "Address::Group(Group")
                    .replace("addresses: [", "addresses: vec![")
                    .replace("\")", "\".into())")
            );*/

            /*println!(
                "{} ->\n{:?}\n{}\n",
                input.0.escape_debug(),
                result,
                "-".repeat(50)
            );*/

            assert_eq!(result, input.1, "Failed for '{:?}'", input.0);
        }
    }
}
