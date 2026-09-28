/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::Resolver;
use crate::store::AddressEntry;
use std::fmt;

/// An address list: mailboxes and groups in document order (issue #98).
/// Lists compare by content.
///
/// ```
/// use mail_parser::{Address, MessageParser};
///
/// let raw = b"To: Ann <ann@example.com>, team: bob@example.com, cy@example.com;\r\n\r\n";
/// let message = MessageParser::new().parse(raw).expect("message");
/// let to = message.to().expect("addresses");
/// let addresses: Vec<_> = to.mailboxes().filter_map(|mailbox| mailbox.address()).collect();
/// assert_eq!(addresses, ["ann@example.com", "bob@example.com", "cy@example.com"]);
/// assert!(matches!(to.iter().nth(1), Some(Address::Group(group)) if group.name() == Some("team")));
/// assert!(to.contains("BOB@example.com"));
/// ```
#[derive(Clone, Copy)]
pub struct AddressList<'m> {
    resolver: Resolver<'m>,
    items: &'m [AddressEntry],
}

/// One item of an address list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address<'m> {
    /// A mailbox outside any group.
    Mailbox(Mailbox<'m>),
    /// A group and its mailboxes.
    Group(Group<'m>),
}

/// A mailbox: a display name and an address (or a URL in RFC 2369 list
/// headers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mailbox<'m> {
    name: Option<&'m str>,
    address: Option<&'m str>,
}

/// A named group of mailboxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group<'m> {
    name: Option<&'m str>,
    members: AddressRun<'m>,
}

/// A run of consecutive mailboxes: the members of a [`Group`], or a run
/// yielded by [`AddressList::groups`]. It is an iterator of [`Mailbox`], and
/// runs compare by content.
#[derive(Clone, Copy)]
pub struct AddressRun<'m> {
    resolver: Resolver<'m>,
    items: &'m [AddressEntry],
}

impl fmt::Debug for AddressList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl fmt::Debug for AddressRun<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

impl PartialEq for AddressList<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.items.len() == other.items.len() && self.iter().eq(other.iter())
    }
}

impl Eq for AddressList<'_> {}

impl PartialEq for AddressRun<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.items.len() == other.items.len() && Iterator::eq(*self, *other)
    }
}

impl Eq for AddressRun<'_> {}

impl<'m> Resolver<'m> {
    fn mailbox(self, entry: &AddressEntry) -> Mailbox<'m> {
        Mailbox {
            name: self.opt(entry.name),
            address: self.opt(entry.address),
        }
    }
}

impl<'m> AddressList<'m> {
    pub(crate) fn new(resolver: Resolver<'m>, items: &'m [AddressEntry]) -> Self {
        AddressList { resolver, items }
    }

    /// Whether the list has neither mailboxes nor groups.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Mailboxes and groups in document order.
    pub fn iter(&self) -> impl Iterator<Item = Address<'m>> + use<'m> {
        let resolver = self.resolver;
        let mut rest = self.items;
        std::iter::from_fn(move || {
            let (first, tail) = rest.split_first()?;
            if first.is_group() {
                let (members, tail) = tail.split_at(first.members().min(tail.len()));
                rest = tail;
                Some(Address::Group(Group {
                    name: resolver.opt(first.name),
                    members: AddressRun {
                        resolver,
                        items: members,
                    },
                }))
            } else {
                rest = tail;
                Some(Address::Mailbox(resolver.mailbox(first)))
            }
        })
    }

    /// Every mailbox, groups flattened.
    pub fn mailboxes(&self) -> impl DoubleEndedIterator<Item = Mailbox<'m>> + use<'m> {
        let resolver = self.resolver;
        self.items
            .iter()
            .filter(|entry| !entry.is_group())
            .map(move |entry| resolver.mailbox(entry))
    }

    /// The first mailbox, groups flattened.
    pub fn first(&self) -> Option<Mailbox<'m>> {
        self.mailboxes().next()
    }

    /// The last mailbox, groups flattened.
    pub fn last(&self) -> Option<Mailbox<'m>> {
        self.mailboxes().next_back()
    }

    /// The JMAP `asGroupedAddresses` shape (RFC 8621): each group with its
    /// name, and each run of mailboxes outside groups as an unnamed group.
    pub fn groups(&self) -> impl Iterator<Item = (Option<&'m str>, AddressRun<'m>)> + use<'m> {
        let resolver = self.resolver;
        let mut rest = self.items;
        std::iter::from_fn(move || {
            let (first, tail) = rest.split_first()?;
            let (name, run, tail) = if first.is_group() {
                let (members, tail) = tail.split_at(first.members().min(tail.len()));
                (resolver.opt(first.name), members, tail)
            } else {
                let len = rest.iter().take_while(|entry| !entry.is_group()).count();
                let (run, tail) = rest.split_at(len);
                (None, run, tail)
            };
            rest = tail;
            Some((
                name,
                AddressRun {
                    resolver,
                    items: run,
                },
            ))
        })
    }

    /// Whether any mailbox has this address (ASCII case-insensitive).
    pub fn contains(&self, address: &str) -> bool {
        self.mailboxes().any(|mailbox| {
            mailbox
                .address
                .is_some_and(|a| a.eq_ignore_ascii_case(address))
        })
    }

    /// Whether the list contains at least one group.
    pub fn has_groups(&self) -> bool {
        self.items.iter().any(AddressEntry::is_group)
    }
}

impl<'m> Mailbox<'m> {
    /// The display name, decoded and unquoted.
    pub fn name(&self) -> Option<&'m str> {
        self.name
    }

    /// The address (`local@domain`), or the URL of an RFC 2369 list header.
    pub fn address(&self) -> Option<&'m str> {
        self.address
    }
}

impl<'m> Group<'m> {
    /// The group name, decoded and unquoted.
    pub fn name(&self) -> Option<&'m str> {
        self.name
    }

    /// The members of the group.
    pub fn mailboxes(&self) -> AddressRun<'m> {
        self.members
    }

    /// Number of members.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the group has no member.
    pub fn is_empty(&self) -> bool {
        self.members.len() == 0
    }
}

impl<'m> Iterator for AddressRun<'m> {
    type Item = Mailbox<'m>;

    fn next(&mut self) -> Option<Mailbox<'m>> {
        let (first, rest) = self.items.split_first()?;
        self.items = rest;
        Some(self.resolver.mailbox(first))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items.len(), Some(self.items.len()))
    }
}

impl DoubleEndedIterator for AddressRun<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (last, rest) = self.items.split_last()?;
        self.items = rest;
        Some(self.resolver.mailbox(last))
    }
}

impl ExactSizeIterator for AddressRun<'_> {}
