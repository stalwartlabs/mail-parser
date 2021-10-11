use std::borrow::Cow;

use crate::parsers::{
    encoded_word::{parse_encoded_word, Rfc2047Parser},
    message_stream::MessageStream,
};

#[derive(PartialEq, Debug)]

pub struct Mailbox<'x> {
    name: Cow<'x, str>,
    email: Cow<'x, str>,
}

#[derive(PartialEq, Debug)]
pub struct AdressWithComment<'x> {
    comment: Cow<'x, str>,
    email: Cow<'x, str>,
}

#[derive(PartialEq, Debug)]
pub struct MailboxWithComment<'x> {
    name: Cow<'x, str>,
    comment: Cow<'x, str>,
    email: Cow<'x, str>,
}

#[derive(PartialEq, Debug)]
pub enum Address<'x> {
    Email(Cow<'x, str>),
    EmailWithComment(AdressWithComment<'x>),
    Mailbox(Mailbox<'x>),
    MailboxWithComment(MailboxWithComment<'x>),
    Invalid(Cow<'x, str>),
}

#[derive(PartialEq, Debug)]
pub struct Group<'x> {
    name: Cow<'x, str>,
    list: AddressList<'x>,
}

#[derive(PartialEq, Debug)]
pub struct GroupWithComment<'x> {
    name: Cow<'x, str>,
    comment: Cow<'x, str>,
    list: AddressList<'x>,
}

pub type AddressList<'x> = Vec<Address<'x>>;

#[derive(PartialEq, Debug)]
pub enum AddressField<'x> {
    Address(Address<'x>),
    List(AddressList<'x>),
    Group(Group<'x>),
    GroupWithComment(GroupWithComment<'x>),
    Empty,
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
    is_group_end: bool,
    is_escaped: bool,

    name_tokens: Vec<Cow<'x, str>>,
    mail_tokens: Vec<Cow<'x, str>>,
    comment_tokens: Vec<Cow<'x, str>>,

    state: AddressState,
    state_stack: Vec<AddressState>,

    addresses: AddressList<'x>,
    group_name: Option<Cow<'x, str>>,
    group_comment: Option<Cow<'x, str>>,
}

impl<'x> AddressParser<'x> {
    pub fn new() -> AddressParser<'x> {
        AddressParser {
            token_start: 0,
            token_end: 0,

            is_token_safe: true,
            is_token_email: false,
            is_token_start: true,
            is_group_end: false,
            is_escaped: false,

            name_tokens: Vec::with_capacity(3),
            mail_tokens: Vec::with_capacity(3),
            comment_tokens: Vec::with_capacity(3),

            state: AddressState::Name,
            state_stack: Vec::with_capacity(5),

            addresses: AddressList::new(),
            group_name: None,
            group_comment: None,
        }
    }
}

pub fn add_token<'x>(
    mut parser: AddressParser<'x>,
    stream: &'x MessageStream,
) -> AddressParser<'x> {
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
        list.push(Cow::from(" "));
    }

    list.push(token);

    parser.token_start = 0;
    parser.is_token_safe = true;
    parser.is_token_email = false;
    parser.is_token_start = true;
    parser.is_escaped = false;
    parser
}

fn concat_tokens<'x>(tokens: &mut Vec<Cow<'x, str>>) -> Cow<'x, str> {
    let result;
    if tokens.len() == 1 {
        result = tokens.pop().unwrap();
    } else {
        result = Cow::from(tokens.concat());
        tokens.clear();
    }
    result
}

pub fn add_address(mut parser: AddressParser) -> AddressParser {
    let has_mail = !parser.mail_tokens.is_empty();
    let has_name = !parser.name_tokens.is_empty();
    let has_comment = !parser.comment_tokens.is_empty();

    if has_mail && has_name && has_comment {
        parser
            .addresses
            .push(Address::MailboxWithComment(MailboxWithComment {
                name: concat_tokens(&mut parser.name_tokens),
                comment: concat_tokens(&mut parser.comment_tokens),
                email: concat_tokens(&mut parser.mail_tokens),
            }));
    } else if has_name && has_mail {
        parser.addresses.push(Address::Mailbox(Mailbox {
            name: concat_tokens(&mut parser.name_tokens),
            email: concat_tokens(&mut parser.mail_tokens),
        }));
    } else if has_mail && has_comment {
        parser
            .addresses
            .push(Address::EmailWithComment(AdressWithComment {
                comment: concat_tokens(&mut parser.comment_tokens),
                email: concat_tokens(&mut parser.mail_tokens),
            }));
    } else if has_mail {
        parser
            .addresses
            .push(Address::Email(concat_tokens(&mut parser.mail_tokens)));
    } else if has_name && has_comment {
        parser.addresses.push(Address::Invalid(
            concat_tokens(&mut parser.name_tokens)
                + " "
                + concat_tokens(&mut parser.comment_tokens),
        ));
    } else if has_name {
        parser
            .addresses
            .push(Address::Invalid(concat_tokens(&mut parser.name_tokens)));
    } else if has_comment {
        parser
            .addresses
            .push(Address::Invalid(concat_tokens(&mut parser.comment_tokens)));
    }

    parser
}

pub fn add_group(mut parser: AddressParser) -> AddressParser {
    let has_mail = !parser.mail_tokens.is_empty();
    let has_name = !parser.name_tokens.is_empty();
    let has_comment = !parser.comment_tokens.is_empty();

    if has_name && has_mail {
        parser.group_name = Some(
            concat_tokens(&mut parser.name_tokens) + " " + concat_tokens(&mut parser.mail_tokens),
        );
    } else if has_name {
        parser.group_name = Some(concat_tokens(&mut parser.name_tokens));
    } else if has_mail {
        parser.group_name = Some(concat_tokens(&mut parser.mail_tokens));
    }
    if has_comment {
        parser.group_comment = Some(concat_tokens(&mut parser.comment_tokens));
    }

    parser
}

pub fn parse_address<'x>(stream: &'x MessageStream) -> AddressField<'x> {
    let mut parser = AddressParser::new();

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }

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
                    parser = add_token(parser, stream);
                }

                parser.is_escaped = true;
                continue;
            }
            b',' if parser.state == AddressState::Name => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }
                parser = add_address(parser);
                continue;
            }
            b'<' if parser.state == AddressState::Name => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }
                parser.state_stack.push(AddressState::Name);
                parser.state = AddressState::Address;
                continue;
            }
            b'>' if parser.state == AddressState::Address => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }
                parser.state = parser.state_stack.pop().unwrap();
                continue;
            }
            b'"' if !parser.is_escaped => match parser.state {
                AddressState::Name => {
                    parser.state_stack.push(AddressState::Name);
                    parser.state = AddressState::Quote;
                    if parser.token_start > 0 {
                        parser = add_token(parser, stream);
                    }
                    continue;
                }
                AddressState::Quote => {
                    if parser.token_start > 0 {
                        parser = add_token(parser, stream);
                    }
                    parser.state = parser.state_stack.pop().unwrap();
                    continue;
                }
                _ => (),
            },
            b'@' if parser.state == AddressState::Name => {
                parser.is_token_email = true;
            }
            b'=' if parser.is_token_start && !parser.is_escaped && stream.skip_byte(&b'?') => {
                let pos_back = stream.get_pos() - 1;
                if let Some(token) = parse_encoded_word(stream) {
                    let add_space = if parser.token_start > 0 {
                        parser = add_token(parser, stream);
                        parser.state != AddressState::Quote
                    } else {
                        false
                    };
                    let list = if parser.state != AddressState::Comment {
                        &mut parser.name_tokens
                    } else {
                        &mut parser.comment_tokens
                    };
                    if add_space {
                        list.push(" ".into());
                    }
                    list.push(token.into());
                    continue;
                }
                stream.set_pos(pos_back);
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
                    if parser.token_start > 0 {
                        parser = add_token(parser, stream);
                    }
                    parser.state = AddressState::Comment;
                    continue;
                }
            }
            b')' if parser.state == AddressState::Comment && !parser.is_escaped => {
                let new_state = parser.state_stack.pop().unwrap();
                if parser.state != new_state {
                    if parser.token_start > 0 {
                        parser = add_token(parser, stream);
                    }
                    parser.state = new_state;
                    continue;
                }
            }
            b':' if parser.state == AddressState::Name && !parser.is_escaped => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }
                parser = add_group(parser);
                continue;
            }
            b';' if parser.state == AddressState::Name => {
                if parser.token_start > 0 {
                    parser = add_token(parser, stream);
                }
                parser = add_address(parser);
                parser.is_group_end = true;
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

    if !parser.is_group_end {
        parser = add_address(parser);
    } else {
        parser = add_group(parser);
    }

    if parser.group_name.is_none() {
        if !parser.addresses.is_empty() {
            if parser.addresses.len() == 1 {
                AddressField::Address(parser.addresses.pop().unwrap())
            } else {
                AddressField::List(parser.addresses)
            }
        } else {
            AddressField::Empty
        }
    } else if parser.group_comment.is_none() {
        AddressField::Group(Group {
            name: parser.group_name.unwrap(),
            list: parser.addresses,
        })
    } else {
        AddressField::GroupWithComment(GroupWithComment {
            name: parser.group_name.unwrap(),
            comment: parser.group_comment.unwrap(),
            list: parser.addresses,
        })
    }    
}

mod tests {
    use crate::parsers::{fields::address::parse_address, message_stream::MessageStream};

    use super::*;

    #[test]
    fn parse_addresses() {
        let inputs = [
            (
                "John Doe <jdoe@machine.example>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("John Doe"), email: Cow::from("jdoe@machine.example") }))),
            (
                " Mary Smith <mary@example.net>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Mary Smith"), email: Cow::from("mary@example.net") }))),
            (
                "\"Joe Q. Public\" <john.q.public@example.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Joe Q. Public"), email: Cow::from("john.q.public@example.com") }))),
            (
                "Mary Smith <mary@x.test>, jdoe@example.org, Who? <one@y.test>\n".to_string(), 
                AddressField::List(AddressList::from([Address::Mailbox(Mailbox { name: Cow::from("Mary Smith"), email: Cow::from("mary@x.test") }), Address::Email(Cow::from("jdoe@example.org")), Address::Mailbox(Mailbox { name: Cow::from("Who?"), email: Cow::from("one@y.test") })]))),
            (
                "<boss@nil.test>, \"Giant; \\\"Big\\\" Box\" <sysservices@example.net>\n".to_string(), 
                AddressField::List(AddressList::from([Address::Email(Cow::from("boss@nil.test")), Address::Mailbox(Mailbox { name: Cow::from("Giant; \"Big\" Box"), email: Cow::from("sysservices@example.net") })]))),
            (
                "A Group:Ed Jones <c@a.test>,joe@where.test,John <jdoe@one.test>;\n".to_string(), 
                AddressField::Group(Group { name: Cow::from("A Group"), list: AddressList::from([Address::Mailbox(Mailbox { name: Cow::from("Ed Jones"), email: Cow::from("c@a.test") }), Address::Email(Cow::from("joe@where.test")), Address::Mailbox(Mailbox { name: Cow::from("John"), email: Cow::from("jdoe@one.test") })]) })),
            (
                "Undisclosed recipients:;\n".to_string(), 
                AddressField::Group(Group { name: Cow::from("Undisclosed recipients"), list: AddressList::from([]) })),
            (
                "\"Mary Smith: Personal Account\" <smith@home.example >\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Mary Smith: Personal Account"), email: Cow::from("smith@home.example") }))),
            (
                "Pete(A nice \\) chap) <pete(his account)@silly.test(his host)>\n".to_string(), 
                AddressField::Address(Address::MailboxWithComment(MailboxWithComment { name: Cow::from("Pete"), comment: Cow::from("A nice ) chap his account his host"), email: Cow::from("pete@silly.test") }))),
            (
                "Pete(A nice \n \\\n ) chap) <pete(his\n account)@silly\n .test(his host)>\n".to_string(), 
                AddressField::Address(Address::MailboxWithComment(MailboxWithComment { name: Cow::from("Pete"), comment: Cow::from("A nice ) chap his account his host"), email: Cow::from("pete@silly.test") }))),
            (
                "A Group(Some people)\n        :Chris Jones <c@(Chris's host.)public.example>,\n            joe@example.org,\n     John <jdoe@one.test> (my dear friend); (the end of the group)\n".to_string(), 
                AddressField::GroupWithComment(GroupWithComment { name: Cow::from("A Group"), comment: Cow::from("the end of the group"), list: AddressList::from([Address::MailboxWithComment(MailboxWithComment { name: Cow::from("Chris Jones"), comment: Cow::from("Chris's host."), email: Cow::from("c@public.example") }), Address::Email(Cow::from("joe@example.org")), Address::MailboxWithComment(MailboxWithComment { name: Cow::from("John"), comment: Cow::from("my dear friend"), email: Cow::from("jdoe@one.test") })]) }),),
            (
                "(Empty list)(start)Hidden recipients  :(nobody(that I know))  ;\n".to_string(), 
                AddressField::GroupWithComment(GroupWithComment { name: Cow::from("Hidden recipients"), comment: Cow::from("Empty list start"), list: AddressList::from([Address::Invalid(Cow::from("nobody(that I know)"))]) })),
            (
                "Joe Q. Public <john.q.public@example.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Joe Q. Public"), email: Cow::from("john.q.public@example.com") }))),
            (
                "Mary Smith <@node.test:mary@example.net>, , jdoe@test  . example\n".to_string(), 
                AddressField::List(AddressList::from([Address::Mailbox(Mailbox { name: Cow::from("Mary Smith"), email: Cow::from("@node.test:mary@example.net") }), Address::Email(Cow::from("jdoe@test  . example"))]))),
            (
                "John Doe <jdoe@machine(comment).  example>\n".to_string(), 
                AddressField::Address(Address::MailboxWithComment(MailboxWithComment { name: Cow::from("John Doe"), comment: Cow::from("comment"), email: Cow::from("jdoe@machine.  example") }))),
            (
                "Mary Smith\n    \n\t<mary@example.net>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Mary Smith"), email: Cow::from("mary@example.net") }))),
            (
                "=?US-ASCII*EN?Q?Keith_Moore?= <moore@cs.utk.edu>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Keith Moore"), email: Cow::from("moore@cs.utk.edu") }))),
            (
                "John =?US-ASCII*EN?Q?Doe?= <moore@cs.utk.edu>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("John Doe"), email: Cow::from("moore@cs.utk.edu") }))),
            (
                "=?ISO-8859-1?Q?Keld_J=F8rn_Simonsen?= <keld@dkuug.dk>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Keld Jørn Simonsen"), email: Cow::from("keld@dkuug.dk") }))),
            (
                "=?ISO-8859-1?Q?Andr=E9?= Pirard <PIRARD@vm1.ulg.ac.be>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("André Pirard"), email: Cow::from("PIRARD@vm1.ulg.ac.be") }))),
            (
                "=?ISO-8859-1?Q?Olle_J=E4rnefors?= <ojarnef@admin.kth.se>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("Olle Järnefors"), email: Cow::from("ojarnef@admin.kth.se") }))),
            (
                "ietf-822@dimacs.rutgers.edu, ojarnef@admin.kth.se\n".to_string(), 
                AddressField::List(AddressList::from([Address::Email(Cow::from("ietf-822@dimacs.rutgers.edu")), Address::Email(Cow::from("ojarnef@admin.kth.se"))]))),
            (
                "Nathaniel Borenstein <nsb@thumper.bellcore.com>\n    (=?iso-8859-8?b?7eXs+SDv4SDp7Oj08A==?=)\n".to_string(), 
                AddressField::Address(Address::MailboxWithComment(MailboxWithComment { name: Cow::from("Nathaniel Borenstein"), comment: Cow::from("םולש ןב ילטפנ"), email: Cow::from("nsb@thumper.bellcore.com") })),),
            (
                "Greg Vaudreuil <gvaudre@NRI.Reston.VA.US>, Ned Freed\n      <ned@innosoft.com>, Keith Moore <moore@cs.utk.edu>\n".to_string(), 
                AddressField::List(AddressList::from([Address::Mailbox(Mailbox { name: Cow::from("Greg Vaudreuil"), email: Cow::from("gvaudre@NRI.Reston.VA.US") }), Address::Mailbox(Mailbox { name: Cow::from("Ned Freed"), email: Cow::from("ned@innosoft.com") }), Address::Mailbox(Mailbox { name: Cow::from("Keith Moore"), email: Cow::from("moore@cs.utk.edu") })]))),
            (
                "=?ISO-8859-1?Q?a?= <test@test.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("a"), email: Cow::from("test@test.com") })),),
            (
                "\"=?ISO-8859-1?Q?a?= b\" <test@test.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("a b"), email: Cow::from("test@test.com") }))),
            (
                "=?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?b?= <test@test.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("ab"), email: Cow::from("test@test.com") }))),
            (
                "=?ISO-8859-1?Q?a?=\n   =?ISO-8859-1?Q?b?= <test@test.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("ab"), email: Cow::from("test@test.com") }))),
            (
                "=?ISO-8859-1?Q?a?= \"=?ISO-8859-2?Q?_b?=\" <test@test.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("a b"), email: Cow::from("test@test.com") }))),
            (
                " <test@test.com>\n".to_string(), 
                AddressField::Address(Address::Email(Cow::from("test@test.com")))),
            (
                "test@test.com\ninvalid@address.com\n".to_string(), 
                AddressField::Address(Address::Email(Cow::from("test@test.com")))),
            (
                "\"=?ISO-8859-1?Q =?ISO-8859-1?Q?a?= \\\" =?ISO-8859-1?Q?b?=\" <last@address.com>\n\nbody@content.com".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("=?ISO-8859-1?Q a \" b"), email: Cow::from("last@address.com") }))),
            (
                "=? <name@domain.com>\n".to_string(), 
                AddressField::Address(Address::Mailbox(Mailbox { name: Cow::from("=?"), email: Cow::from("name@domain.com") }))),
        ];

        for input in inputs {
            let stream = &MessageStream::new(input.0.as_bytes());
            let result = parse_address(stream);
            //println!("{} ->\n{:?}\n{}\n", input.0.escape_debug(), result, "-".repeat(50) );
            assert_eq!(result, input.1, "Failed for {}", input.0.escape_debug());
        }
    }
}
