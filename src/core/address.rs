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

use crate::{Addr, Group};

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
}
