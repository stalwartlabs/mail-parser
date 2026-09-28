/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::ops::{Deref, DerefMut};

pub(crate) trait Blank: Copy {
    const BLANK: Self;
}

impl Blank for u32 {
    const BLANK: u32 = 0;
}

#[derive(Debug, Clone)]
pub(crate) enum Table<T: Blank, const N: usize> {
    Inline([T; N], u32),
    Spill(Vec<T>),
}

impl<T: Blank, const N: usize> Default for Table<T, N> {
    fn default() -> Self {
        Table::EMPTY
    }
}

impl<T: Blank, const N: usize> Table<T, N> {
    pub(crate) const EMPTY: Self = Table::Inline([T::BLANK; N], 0);

    #[inline]
    pub(crate) fn push(&mut self, item: T, first: usize) {
        match self {
            Table::Inline(items, len) => match items.get_mut(*len as usize) {
                Some(slot) => {
                    *slot = item;
                    *len += 1;
                }
                None => self.spill(item, first),
            },
            Table::Spill(spill) => spill.push(item),
        }
    }

    #[inline(never)]
    fn spill(&mut self, item: T, first: usize) {
        let mut spill = Vec::with_capacity(first.max(N + 1));
        spill.extend_from_slice(self);
        spill.push(item);
        *self = Table::Spill(spill);
    }

    pub(crate) fn extend_from_slice(&mut self, items: &[T], first: usize) {
        match self {
            Table::Spill(spill) => spill.extend_from_slice(items),
            Table::Inline(..) => items.iter().for_each(|item| self.push(*item, first)),
        }
    }

    pub(crate) fn resize(&mut self, new_len: usize, value: T, first: usize) {
        if new_len <= self.len() {
            self.truncate(new_len);
            return;
        }
        match self {
            Table::Spill(spill) => spill.resize(new_len, value),
            Table::Inline(..) => (self.len()..new_len).for_each(|_| self.push(value, first)),
        }
    }

    pub(crate) fn truncate(&mut self, new_len: usize) {
        match self {
            Table::Inline(_, len) => *len = (*len).min(u32::try_from(new_len).unwrap_or(u32::MAX)),
            Table::Spill(spill) => spill.truncate(new_len),
        }
    }

    pub(crate) fn clear(&mut self) {
        match self {
            Table::Inline(_, len) => *len = 0,
            Table::Spill(spill) => spill.clear(),
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        match self {
            Table::Inline(_, len) => *len as usize,
            Table::Spill(spill) => spill.len(),
        }
    }

    pub(crate) fn capacity_bytes(&self) -> usize {
        match self {
            Table::Inline(..) => 0,
            Table::Spill(spill) => spill.capacity() * size_of::<T>(),
        }
    }

    #[cfg(test)]
    pub(crate) fn is_spilled(&self) -> bool {
        matches!(self, Table::Spill(_))
    }
}

impl<T: Blank, const N: usize> Deref for Table<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        match self {
            Table::Inline(items, len) => items.get(..*len as usize).unwrap_or_default(),
            Table::Spill(spill) => spill,
        }
    }
}

impl<T: Blank, const N: usize> DerefMut for Table<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            Table::Inline(items, len) => items.get_mut(..*len as usize).unwrap_or_default(),
            Table::Spill(spill) => spill,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Table;

    #[test]
    fn inline_then_spill() {
        let mut table: Table<u32, 2> = Table::default();
        let mut reference = Vec::new();
        for value in 0..40u32 {
            table.push(value, 4);
            reference.push(value);
            assert_eq!(&*table, reference.as_slice());
            assert_eq!(table.is_spilled(), reference.len() > 2);
        }
        table.truncate(5);
        reference.truncate(5);
        assert_eq!(&*table, reference.as_slice());
        table.resize(9, 7, 4);
        reference.resize(9, 7);
        assert_eq!(&*table, reference.as_slice());
        table.clear();
        assert!(table.is_empty() && table.is_spilled());
        table.push(3, 4);
        assert_eq!(&*table, &[3]);
    }

    #[test]
    fn inline_operations() {
        let mut table: Table<u32, 4> = Table::default();
        table.extend_from_slice(&[1, 2, 3], 8);
        assert_eq!(&*table, &[1, 2, 3]);
        assert!(!table.is_spilled());
        table.resize(4, 9, 8);
        assert_eq!(&*table, &[1, 2, 3, 9]);
        assert!(!table.is_spilled());
        if let Some(first) = table.first_mut() {
            *first = 5;
        }
        table.truncate(1);
        assert_eq!(&*table, &[5]);
        table.resize(6, 0, 8);
        assert_eq!(&*table, &[5, 0, 0, 0, 0, 0]);
        assert!(table.is_spilled());
        let mut empty: Table<u32, 1> = Table::default();
        empty.truncate(3);
        empty.clear();
        assert!(empty.is_empty());
    }
}
