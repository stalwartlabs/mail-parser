/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{Addr, Address, Group};

impl<'x> Address<'x> {
    /// Returns the first address in the list, or the first address in the first group.
    pub fn first(&self) -> Option<&Addr<'x>> {
        self.groups
            .iter()
            .flat_map(|group| group.addresses.iter())
            .next()
    }

    /// Returns the last address in the list, or the last address in the last group.
    pub fn last(&self) -> Option<&Addr<'x>> {
        self.groups
            .iter()
            .flat_map(|group| group.addresses.iter())
            .next_back()
    }

    /// Converts the address into a list of `Addr`.
    pub fn into_list(self) -> Vec<Addr<'x>> {
        self.groups
            .into_iter()
            .flat_map(|group| group.addresses)
            .collect()
    }

    /// Converts the address into a group of `Addr`.
    pub fn into_group(self) -> Vec<Group<'x>> {
        self.groups
    }

    /// Returns the group of addresses.
    pub fn as_group(&self) -> &[Group<'x>] {
        &self.groups
    }

    /// Returns an iterator over the addresses in the list, or the addresses in the groups.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Addr<'x>> + '_ + Sync + Send {
        self.groups.iter().flat_map(|group| group.addresses.iter())
    }

    /// Returns whether the list contains the given address.
    pub fn contains(&self, addr: &str) -> bool {
        self.groups.iter().any(|group| {
            group.addresses.iter().any(|a| {
                a.address
                    .as_ref()
                    .is_some_and(|a| a.eq_ignore_ascii_case(addr))
            })
        })
    }

    pub fn into_owned(self) -> Address<'static> {
        let groups = self
            .groups
            .into_iter()
            .map(|group| group.into_owned())
            .collect();
        Address { groups }
    }
}

impl<'x> Group<'x> {
    pub fn new(name: &'x str, addresses: Vec<Addr<'x>>) -> Self {
        Self {
            name: Some(name.into()),
            addresses,
        }
    }

    pub fn into_owned(self) -> Group<'static> {
        Group {
            name: self.name.map(|s| s.into_owned().into()),
            addresses: self.addresses.into_iter().map(|a| a.into_owned()).collect(),
        }
    }
}

impl<'x> Addr<'x> {
    pub fn new(name: Option<&'x str>, address: &'x str) -> Self {
        Self {
            name: name.map(|name| name.into()),
            address: Some(address.into()),
        }
    }

    pub fn into_owned(self) -> Addr<'static> {
        Addr {
            name: self.name.map(|s| s.into_owned().into()),
            address: self.address.map(|s| s.into_owned().into()),
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }
}
