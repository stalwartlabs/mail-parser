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

use std::borrow::Cow;

use crate::{parsers::MessageStream, Addr, Group, HeaderValue};

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

impl<'x> AddressParser<'x> {
    pub fn add_token(&mut self, stream: &MessageStream<'x>, add_trail_space: bool) {
        if self.token_start > 0 {
            let token = String::from_utf8_lossy(&stream.data[self.token_start - 1..self.token_end]);
            let mut add_space = false;
            let list = match self.state {
                AddressState::Address => &mut self.mail_tokens,
                AddressState::Name => {
                    if self.is_token_email {
                        &mut self.mail_tokens
                    } else {
                        add_space = true;
                        &mut self.name_tokens
                    }
                }
                AddressState::Quote => &mut self.name_tokens,
                AddressState::Comment => {
                    add_space = true;
                    &mut self.comment_tokens
                }
            };

            if add_space && !list.is_empty() {
                list.push(" ".into());
            }

            list.push(token);

            if add_trail_space {
                list.push(" ".into());
            }

            self.token_start = 0;
            self.is_token_email = false;
            self.is_token_start = true;
            self.is_escaped = false;
        }
    }

    pub fn add_address(&mut self) {
        let has_mail = !self.mail_tokens.is_empty();
        let has_name = !self.name_tokens.is_empty();
        let has_comment = !self.comment_tokens.is_empty();

        self.addresses.push(if has_mail && has_name && has_comment {
            Addr {
                name: Some(
                    format!(
                        "{} ({})",
                        concat_tokens(&mut self.name_tokens),
                        concat_tokens(&mut self.comment_tokens)
                    )
                    .into(),
                ),
                address: concat_tokens(&mut self.mail_tokens).into(),
            }
        } else if has_name && has_mail {
            Addr {
                name: concat_tokens(&mut self.name_tokens).into(),
                address: concat_tokens(&mut self.mail_tokens).into(),
            }
        } else if has_mail && has_comment {
            Addr {
                name: concat_tokens(&mut self.comment_tokens).into(),
                address: concat_tokens(&mut self.mail_tokens).into(),
            }
        } else if has_mail {
            Addr {
                name: None,
                address: concat_tokens(&mut self.mail_tokens).into(),
            }
        } else if has_name && has_comment {
            Addr {
                name: concat_tokens(&mut self.comment_tokens).into(),
                address: concat_tokens(&mut self.name_tokens).into(),
            }
        } else if has_name {
            Addr {
                name: concat_tokens(&mut self.name_tokens).into(),
                address: None,
            }
        } else if has_comment {
            Addr {
                name: concat_tokens(&mut self.comment_tokens).into(),
                address: None,
            }
        } else {
            return;
        });
    }

    pub fn add_group_details(&mut self) {
        if !self.name_tokens.is_empty() {
            self.group_name = concat_tokens(&mut self.name_tokens).into();
        }

        if !self.comment_tokens.is_empty() {
            self.group_comment = concat_tokens(&mut self.comment_tokens).into();
        }

        if !self.mail_tokens.is_empty() {
            if self.group_name.is_none() {
                self.group_name = concat_tokens(&mut self.mail_tokens).into();
            } else {
                self.group_name = Some(
                    (self.group_name.as_ref().unwrap().as_ref().to_owned()
                        + " "
                        + concat_tokens(&mut self.mail_tokens).as_ref())
                    .into(),
                );
            }
        }
    }

    pub fn add_group(&mut self) {
        let has_name = self.group_name.is_some();
        let has_comment = self.group_comment.is_some();
        let has_addresses = !self.addresses.is_empty();

        self.result
            .push(if has_name && has_addresses && has_comment {
                Group {
                    name: Some(
                        format!(
                            "{} ({})",
                            self.group_name.take().unwrap(),
                            self.group_comment.take().unwrap()
                        )
                        .into(),
                    ),
                    addresses: std::mem::take(&mut self.addresses),
                }
            } else if has_addresses && has_name {
                Group {
                    name: self.group_name.take(),
                    addresses: std::mem::take(&mut self.addresses),
                }
            } else if has_addresses {
                Group {
                    name: self.group_comment.take(),
                    addresses: std::mem::take(&mut self.addresses),
                }
            } else if has_name {
                Group {
                    name: self.group_name.take(),
                    addresses: Vec::new(),
                }
            } else {
                return;
            });
    }
}

impl<'x> MessageStream<'x> {
    pub fn parse_address(&mut self) -> HeaderValue<'x> {
        let mut parser = AddressParser {
            token_start: 0,
            token_end: 0,

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

        while let Some(ch) = self.next() {
            match ch {
                b'\n' => {
                    parser.add_token(self, false);
                    if self.try_next_is_space() {
                        if !parser.is_token_start {
                            parser.is_token_start = true;
                        }
                        continue;
                    } else {
                        break;
                    }
                }
                b'\\' if parser.state != AddressState::Name && !parser.is_escaped => {
                    if parser.token_start > 0 {
                        if parser.state == AddressState::Quote {
                            parser.token_end = self.offset() - 1;
                        }
                        parser.add_token(self, false);
                    }
                    parser.is_escaped = true;
                    continue;
                }
                b',' if parser.state == AddressState::Name => {
                    parser.add_token(self, false);
                    parser.add_address();
                    continue;
                }
                b'<' if parser.state == AddressState::Name => {
                    parser.add_token(self, false);
                    parser.state_stack.push(AddressState::Name);
                    parser.state = AddressState::Address;
                    continue;
                }
                b'>' if parser.state == AddressState::Address => {
                    parser.add_token(self, false);
                    parser.state = parser.state_stack.pop().unwrap();
                    continue;
                }
                b'"' if !parser.is_escaped => match parser.state {
                    AddressState::Name => {
                        parser.state_stack.push(AddressState::Name);
                        parser.state = AddressState::Quote;
                        parser.add_token(self, false);
                        continue;
                    }
                    AddressState::Quote => {
                        parser.add_token(self, false);
                        parser.state = parser.state_stack.pop().unwrap();
                        continue;
                    }
                    _ => (),
                },
                b'@' if parser.state == AddressState::Name => {
                    parser.is_token_email = true;
                }
                b'=' if parser.is_token_start && !parser.is_escaped && self.peek_char(b'?') => {
                    self.checkpoint();
                    if let Some(token) = self.decode_rfc2047() {
                        let add_space = parser.state != AddressState::Quote; // Make borrow-checker happy
                        parser.add_token(self, add_space);
                        (if parser.state != AddressState::Comment {
                            &mut parser.name_tokens
                        } else {
                            &mut parser.comment_tokens
                        })
                        .push(token.into());
                        continue;
                    }
                    self.restore();
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
                            parser.token_start = self.offset();
                            parser.token_end = parser.token_start;
                        } else {
                            parser.token_end = self.offset();
                        }
                    }
                    continue;
                }
                b'\r' => continue,
                b'(' if parser.state != AddressState::Quote && !parser.is_escaped => {
                    parser.state_stack.push(parser.state);
                    if parser.state != AddressState::Comment {
                        parser.add_token(self, false);
                        parser.state = AddressState::Comment;
                        continue;
                    }
                }
                b')' if parser.state == AddressState::Comment && !parser.is_escaped => {
                    let new_state = parser.state_stack.pop().unwrap();
                    if parser.state != new_state {
                        parser.add_token(self, false);
                        parser.state = new_state;
                        continue;
                    }
                }
                b':' if parser.state == AddressState::Name && !parser.is_escaped => {
                    parser.add_group();
                    parser.add_token(self, false);
                    parser.add_group_details();
                    continue;
                }
                b';' if parser.state == AddressState::Name => {
                    parser.add_token(self, false);
                    parser.add_address();
                    parser.add_group();
                    continue;
                }
                _ => (),
            }

            if parser.is_escaped {
                parser.is_escaped = false;
            }

            if parser.is_token_start {
                parser.is_token_start = false;
            }

            if parser.token_start == 0 {
                parser.token_start = self.offset();
                parser.token_end = parser.token_start;
            } else {
                parser.token_end = self.offset();
            }
        }

        parser.add_address();

        if parser.group_name.is_some() || !parser.result.is_empty() {
            parser.add_group();
            if parser.result.len() > 1 {
                HeaderValue::GroupList(parser.result)
            } else {
                HeaderValue::Group(parser.result.pop().unwrap())
            }
        } else if !parser.addresses.is_empty() {
            if parser.addresses.len() > 1 {
                HeaderValue::AddressList(parser.addresses)
            } else {
                HeaderValue::Address(parser.addresses.pop().unwrap())
            }
        } else {
            HeaderValue::Empty
        }
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

pub fn parse_address_local_part(addr: &str) -> Option<&str> {
    let addr = addr.as_bytes();
    let mut iter = addr.iter().enumerate();
    while let Some((pos, &ch)) = iter.next() {
        if ch == b'@' {
            return if pos > 0 && iter.next().is_some() {
                std::str::from_utf8(addr.get(..pos)?).ok()
            } else {
                None
            };
        } else if !ch.is_ascii() {
            return None;
        }
    }

    None
}

pub fn parse_address_domain(addr: &str) -> Option<&str> {
    let addr = addr.as_bytes();
    for (pos, &ch) in addr.iter().enumerate() {
        if ch == b'@' {
            return if pos > 0 && pos + 1 < addr.len() {
                std::str::from_utf8(addr.get(pos + 1..)?).ok()
            } else {
                None
            };
        } else if !ch.is_ascii() {
            return None;
        }
    }

    None
}

pub fn parse_address_user_part(addr: &str) -> Option<&str> {
    let addr = addr.as_bytes();

    let mut iter = addr.iter().enumerate();
    while let Some((pos, &ch)) = iter.next() {
        if ch == b'+' {
            if pos > 0 {
                while let Some((_, &ch)) = iter.next() {
                    if ch == b'@' && iter.next().is_some() {
                        return std::str::from_utf8(addr.get(..pos)?).ok();
                    }
                }
            }
            return None;
        } else if ch == b'@' {
            return if pos > 0 && iter.next().is_some() {
                std::str::from_utf8(addr.get(..pos)?).ok()
            } else {
                None
            };
        } else if !ch.is_ascii() {
            return None;
        }
    }

    None
}

pub fn parse_address_detail_part(addr: &str) -> Option<&str> {
    let addr = addr.as_bytes();
    let mut plus_pos = usize::MAX;

    let mut iter = addr.iter().enumerate();
    while let Some((pos, &ch)) = iter.next() {
        if ch == b'+' {
            plus_pos = pos + 1;
        } else if ch == b'@' {
            if plus_pos != usize::MAX && iter.next().is_some() {
                return std::str::from_utf8(addr.get(plus_pos..pos)?).ok();
            } else {
                return None;
            }
        } else if !ch.is_ascii() {
            return None;
        }
    }

    None
}

mod tests {
    #[test]
    fn parse_addresses() {
        use super::*;

        let inputs = [
            (
                "John Doe <jdoe@machine.example>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: John Doe\n",
                    "  address: jdoe@machine.example\n"
                ),
            ),
            (
                " Mary Smith <mary@example.net>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Mary Smith\n",
                    "  address: mary@example.net\n"
                ),
            ),
            (
                "\"Joe Q. Public\" <john.q.public@example.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Joe Q. Public\n",
                    "  address: john.q.public@example.com\n"
                ),
            ),
            (
                "Mary Smith <mary@x.test>, jdoe@example.org, Who? <one@y.test>\n",
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - name: Mary Smith\n",
                    "    address: mary@x.test\n",
                    "  - address: jdoe@example.org\n",
                    "  - name: Who?\n",
                    "    address: one@y.test\n"
                ),
            ),
            (
                "<boss@nil.test>, \"Giant; \\\"Big\\\" Box\" <sysservices@example.net>\n",
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - address: boss@nil.test\n",
                    "  - name: \"Giant; \\\"Big\\\" Box\"\n",
                    "    address: sysservices@example.net\n"
                ),
            ),
            (
                "A Group:Ed Jones <c@a.test>,joe@where.test,John <jdoe@one.test>;\n",
                concat!(
                    "---\n",
                    "Group:\n",
                    "  name: A Group\n",
                    "  addresses:\n",
                    "    - name: Ed Jones\n",
                    "      address: c@a.test\n",
                    "    - address: joe@where.test\n",
                    "    - name: John\n",
                    "      address: jdoe@one.test\n"
                ),
            ),
            (
                "Undisclosed recipients:;\n",
                concat!("---\n", "Group:\n", "  name: Undisclosed recipients\n"),
            ),
            (
                "\"Mary Smith: Personal Account\" <smith@home.example >\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: \"Mary Smith: Personal Account\"\n",
                    "  address: smith@home.example\n"
                ),
            ),
            (
                "Pete(A nice \\) chap) <pete(his account)@silly.test(his host)>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Pete (A nice ) chap his account his host)\n",
                    "  address: pete@silly.test\n"
                ),
            ),
            (
                "Pete(A nice \n \\\n ) chap) <pete(his\n account)@silly\n .test(his host)>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Pete (A nice ) chap his account his host)\n",
                    "  address: pete@silly.test\n"
                ),
            ),
            (
                concat!(
                    "A Group(Some people)\n        :Chris Jones <c@(Chris's host.)public.exa",
                    "mple>,\n            joe@example.org,\n     John <jdoe@one.test> (my dear",
                    " friend); (the end of the group)\n"
                ),
                concat!(
                    "---\n",
                    "GroupList:\n",
                    "  - name: A Group (Some people)\n",
                    "    addresses:\n",
                    "      - name: \"Chris Jones (Chris's host.)\"\n",
                    "        address: c@public.example\n",
                    "      - address: joe@example.org\n",
                    "      - name: John (my dear friend)\n",
                    "        address: jdoe@one.test\n",
                    "  - addresses:\n",
                    "      - name: the end of the group\n"
                ),
            ),
            (
                "(Empty list)(start)Hidden recipients  :(nobody(that I know))  ;\n",
                concat!(
                    "---\n",
                    "Group:\n",
                    "  name: Hidden recipients (Empty list start)\n",
                    "  addresses:\n",
                    "    - name: nobody(that I know)\n"
                ),
            ),
            (
                "Joe Q. Public <john.q.public@example.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Joe Q. Public\n",
                    "  address: john.q.public@example.com\n"
                ),
            ),
            (
                "Mary Smith <@node.test:mary@example.net>, , jdoe@test  . example\n",
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - name: Mary Smith\n",
                    "    address: \"@node.test:mary@example.net\"\n",
                    "  - address: jdoe@test  . example\n"
                ),
            ),
            (
                "John Doe <jdoe@machine(comment).  example>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: John Doe (comment)\n",
                    "  address: jdoe@machine.  example\n"
                ),
            ),
            (
                "Mary Smith\n    \n\t<mary@example.net>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Mary Smith\n",
                    "  address: mary@example.net\n"
                ),
            ),
            (
                "=?US-ASCII*EN?Q?Keith_Moore?= <moore@cs.utk.edu>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Keith Moore\n",
                    "  address: moore@cs.utk.edu\n"
                ),
            ),
            (
                "John =?US-ASCII*EN?Q?Doe?= <moore@cs.utk.edu>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: John Doe\n",
                    "  address: moore@cs.utk.edu\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?Keld_J=F8rn_Simonsen?= <keld@dkuug.dk>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Keld Jørn Simonsen\n",
                    "  address: keld@dkuug.dk\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?Andr=E9?= Pirard <PIRARD@vm1.ulg.ac.be>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: André Pirard\n",
                    "  address: PIRARD@vm1.ulg.ac.be\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?Olle_J=E4rnefors?= <ojarnef@admin.kth.se>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Olle Järnefors\n",
                    "  address: ojarnef@admin.kth.se\n"
                ),
            ),
            (
                "ietf-822@dimacs.rutgers.edu, ojarnef@admin.kth.se\n",
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - address: ietf-822@dimacs.rutgers.edu\n",
                    "  - address: ojarnef@admin.kth.se\n"
                ),
            ),
            (
                concat!(
                    "Nathaniel Borenstein <nsb@thumper.bellcore.com>\n    (=?iso-8859-8?b?7e",
                    "Xs+SDv4SDp7Oj08A==?=)\n"
                ),
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Nathaniel Borenstein (םולש ןב ילטפנ)\n",
                    "  address: nsb@thumper.bellcore.com\n"
                ),
            ),
            (
                concat!(
                    "Greg Vaudreuil <gvaudre@NRI.Reston.VA.US>, Ned Freed\n      <ned@innoso",
                    "ft.com>, Keith Moore <moore@cs.utk.edu>\n"
                ),
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - name: Greg Vaudreuil\n",
                    "    address: gvaudre@NRI.Reston.VA.US\n",
                    "  - name: Ned Freed\n",
                    "    address: ned@innosoft.com\n",
                    "  - name: Keith Moore\n",
                    "    address: moore@cs.utk.edu\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?a?= <test@test.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: a\n",
                    "  address: test@test.com\n"
                ),
            ),
            (
                "\"=?ISO-8859-1?Q?a?= b\" <test@test.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: a b\n",
                    "  address: test@test.com\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?b?= <test@test.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: ab\n",
                    "  address: test@test.com\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?a?=\n   =?ISO-8859-1?Q?b?= <test@test.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: ab\n",
                    "  address: test@test.com\n"
                ),
            ),
            (
                "=?ISO-8859-1?Q?a?= \"=?ISO-8859-2?Q?_b?=\" <test@test.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: a b\n",
                    "  address: test@test.com\n"
                ),
            ),
            (
                " <test@test.com>\n",
                concat!("---\n", "Address:\n", "  address: test@test.com\n"),
            ),
            (
                "test@test.com\ninvalid@address.com\n",
                concat!("---\n", "Address:\n", "  address: test@test.com\n"),
            ),
            (
                concat!(
                    "\"=?ISO-8859-1?Q =?ISO-8859-1?Q?a?= \\\" =?ISO-8859-1?Q?b?=\" <last@addres",
                    "s.com>\n\nbody@content.com"
                ),
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: \"=?ISO-8859-1?Q a \\\" b\"\n",
                    "  address: last@address.com\n"
                ),
            ),
            (
                "=? <name@domain.com>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: \"=?\"\n",
                    "  address: name@domain.com\n"
                ),
            ),
            (
                concat!(
                    "\"  James Smythe\" <james@example.com>, Friends:\n  jane@example.com, =?U",
                    "TF-8?Q?John_Sm=C3=AEth?=\n   <john@example.com>;\n"
                ),
                concat!(
                    "---\n",
                    "GroupList:\n",
                    "  - addresses:\n",
                    "      - name: \"  James Smythe\"\n",
                    "        address: james@example.com\n",
                    "  - name: Friends\n",
                    "    addresses:\n",
                    "      - address: jane@example.com\n",
                    "      - name: John Smîth\n",
                    "        address: john@example.com\n"
                ),
            ),
            (
                concat!(
                    "List 1: addr1@test.com, addr2@test.com; List 2: addr3@test.com, addr4@",
                    "test.com; addr5@test.com, addr6@test.com\n"
                ),
                concat!(
                    "---\n",
                    "GroupList:\n",
                    "  - name: List 1\n",
                    "    addresses:\n",
                    "      - address: addr1@test.com\n",
                    "      - address: addr2@test.com\n",
                    "  - name: List 2\n",
                    "    addresses:\n",
                    "      - address: addr3@test.com\n",
                    "      - address: addr4@test.com\n",
                    "  - addresses:\n",
                    "      - address: addr5@test.com\n",
                    "      - address: addr6@test.com\n"
                ),
            ),
            (
                concat!(
                    "\"List 1\": addr1@test.com, addr2@test.com; \"List 2\": addr3@test.com, ad",
                    "dr4@test.com; addr5@test.com, addr6@test.com\n"
                ),
                concat!(
                    "---\n",
                    "GroupList:\n",
                    "  - name: List 1\n",
                    "    addresses:\n",
                    "      - address: addr1@test.com\n",
                    "      - address: addr2@test.com\n",
                    "  - name: List 2\n",
                    "    addresses:\n",
                    "      - address: addr3@test.com\n",
                    "      - address: addr4@test.com\n",
                    "  - addresses:\n",
                    "      - address: addr5@test.com\n",
                    "      - address: addr6@test.com\n"
                ),
            ),
            (
                concat!(
                    "\"=?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=\": addr1@test.com, addr2@",
                    "test.com; =?utf-8?b?VGjDrXMgw61zIHbDoWzDrWQgw5pURjg=?=: addr3@test.com",
                    ", addr4@test.com; addr5@test.com, addr6@test.com\n"
                ),
                concat!(
                    "---\n",
                    "GroupList:\n",
                    "  - name: Thís ís válíd ÚTF8\n",
                    "    addresses:\n",
                    "      - address: addr1@test.com\n",
                    "      - address: addr2@test.com\n",
                    "  - name: Thís ís válíd ÚTF8\n",
                    "    addresses:\n",
                    "      - address: addr3@test.com\n",
                    "      - address: addr4@test.com\n",
                    "  - addresses:\n",
                    "      - address: addr5@test.com\n",
                    "      - address: addr6@test.com\n"
                ),
            ),
            (
                "<http://www.host.com/list/archive/> (Web Archive)\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Web Archive\n",
                    "  address: \"http://www.host.com/list/archive/\"\n"
                ),
            ),
            (
                "<mailto:archive@host.com?subject=index%20list>\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  address: \"mailto:archive@host.com?subject=index%20list\"\n"
                ),
            ),
            (
                "<mailto:moderator@host.com> (Postings are Moderated)\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Postings are Moderated\n",
                    "  address: \"mailto:moderator@host.com\"\n"
                ),
            ),
            (
                concat!(
                    "(Use this command to join the list)\n   <mailto:list-manager@host.com?b",
                    "ody=subscribe%20list>\n"
                ),
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Use this command to join the list\n",
                    "  address: \"mailto:list-manager@host.com?body=subscribe%20list\"\n"
                ),
            ),
            (
                concat!(
                    "<http://www.host.com/list.cgi?cmd=sub&lst=list>,\n   <mailto:list-manag",
                    "er@host.com?body=subscribe%20list>\n"
                ),
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - address: \"http://www.host.com/list.cgi?cmd=sub&lst=list\"\n",
                    "  - address: \"mailto:list-manager@host.com?body=subscribe%20list\"\n"
                ),
            ),
            (
                "NO (posting not allowed on this list)\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: posting not allowed on this list\n",
                    "  address: \"NO\"\n"
                ),
            ),
            (
                concat!(
                    "<ftp://ftp.host.com/list.txt> (FTP),\n   <mailto:list@host.com?subject=",
                    "help>\n"
                ),
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - name: FTP\n",
                    "    address: \"ftp://ftp.host.com/list.txt\"\n",
                    "  - address: \"mailto:list@host.com?subject=help\"\n"
                ),
            ),
            (
                "<http://www.host.com/list/>, <mailto:list-info@host.com>\n",
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - address: \"http://www.host.com/list/\"\n",
                    "  - address: \"mailto:list-info@host.com\"\n"
                ),
            ),
            (
                concat!(
                    "(Use this command to get off the list)\n     <mailto:list-manager@host.",
                    "com?body=unsubscribe%20list>\n"
                ),
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Use this command to get off the list\n",
                    "  address: \"mailto:list-manager@host.com?body=unsubscribe%20list\"\n"
                ),
            ),
            (
                concat!(
                    "<http://www.host.com/list.cgi?cmd=unsub&lst=list>,\n   <mailto:list-req",
                    "uest@host.com?subject=unsubscribe>\n"
                ),
                concat!(
                    "---\n",
                    "AddressList:\n",
                    "  - address: \"http://www.host.com/list.cgi?cmd=unsub&lst=list\"\n",
                    "  - address: \"mailto:list-request@host.com?subject=unsubscribe\"\n"
                ),
            ),
            (
                "<mailto:listmom@host.com> (Contact Person for Help)\n",
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: Contact Person for Help\n",
                    "  address: \"mailto:listmom@host.com\"\n"
                ),
            ),
            (
                r#""\\\\\\\\S. NIG\\\\\\\\" <first.last@host.com>"#,
                concat!(
                    "---\n",
                    "Address:\n",
                    "  name: \"\\\\\\\\\\\\\\\\S. NIG\\\\\\\\\\\\\\\\\"\n",
                    "  address: first.last@host.com\n",
                ),
            ),
        ];

        for input in inputs {
            let str = input.0.to_string();
            let result = MessageStream::new(str.as_bytes()).parse_address();
            let expected: HeaderValue = serde_yaml::from_str(input.1).unwrap_or(HeaderValue::Empty);

            /*if input.0.len() >= 70 {
                println!(
                    "(concat!({:?}), concat!({:?})),",
                    input
                        .0
                        .chars()
                        .collect::<Vec<char>>()
                        .chunks(70)
                        .map(|c| c.iter().collect::<String>())
                        .collect::<Vec<String>>(),
                    serde_yaml::to_string(&result)
                        .unwrap_or("".to_string())
                        .split_inclusive("\n")
                        .collect::<Vec<&str>>()
                );
            } else {
                println!(
                    "({:?}, concat!({:?})),",
                    input.0,
                    serde_yaml::to_string(&result)
                        .unwrap_or("".to_string())
                        .split_inclusive("\n")
                        .collect::<Vec<&str>>()
                );
            }*/

            assert_eq!(result, expected, "Failed for '{:?}'", input.0);
        }
    }
}
