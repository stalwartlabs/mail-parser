/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(unsafe_code)]

use super::{
    scalar,
    set::{ByteSet, Stop},
};

pub(crate) trait Vector: Copy {
    const BYTES: usize;
    const BITS_PER_BYTE: u32;

    /// Loads `BYTES` bytes from `ptr`, which needs no alignment.
    ///
    /// # Safety
    ///
    /// `ptr` must be valid for reads of `BYTES` bytes, and the running CPU
    /// must support the instruction set of the implementing type.
    unsafe fn load(ptr: *const u8) -> Self;

    /// Copies `byte` into every lane.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instruction set of the implementing
    /// type.
    unsafe fn splat(byte: u8) -> Self;

    /// Sets every bit of the lanes where `self` and `other` are equal.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instruction set of the implementing
    /// type.
    unsafe fn eq(self, other: Self) -> Self;

    /// Bitwise AND.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instruction set of the implementing
    /// type.
    unsafe fn and(self, other: Self) -> Self;

    /// Bitwise OR.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instruction set of the implementing
    /// type.
    unsafe fn or(self, other: Self) -> Self;

    /// The bits of `self` that are clear in `other`.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instruction set of the implementing
    /// type.
    unsafe fn and_not(self, other: Self) -> Self;

    /// Packs each lane of a comparison result into `BITS_PER_BYTE` bits,
    /// lane 0 in the lowest bits.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instruction set of the implementing
    /// type.
    unsafe fn mask(self) -> u64;
}

const UNROLL: usize = 4;
const DASH_LOOKAHEAD: usize = 2;
const FOLD_LOOKAHEAD: usize = 1;
const SET_WIDTH: usize = 16;

#[inline(always)]
fn first<V: Vector>(mask: u64) -> usize {
    (mask.trailing_zeros() / V::BITS_PER_BYTE) as usize
}

/// Vector version of `scalar::dash_line` over the whole of `tail`.
///
/// # Safety
///
/// The running CPU must support the instruction set of `V`.
#[inline(always)]
pub(crate) unsafe fn dash_line<V: Vector>(tail: &[u8]) -> Option<usize> {
    let ptr = tail.as_ptr();
    let mut offset = 0;
    // SAFETY: our caller guarantees the CPU supports `V`. Every load reads
    // `V::BYTES` bytes that lie inside `tail`. The unrolled loop runs while
    // `offset + UNROLL * V::BYTES + DASH_LOOKAHEAD <= tail.len()`; its last
    // loads start at `offset + 1 + 3 * V::BYTES` and at
    // `offset + 3 * V::BYTES + 2`, so they end at most at
    // `offset + 4 * V::BYTES + 2`, within that bound. The single-vector loop
    // runs while `offset + V::BYTES + DASH_LOOKAHEAD <= tail.len()` and loads
    // from `offset`, `offset + 1` and `offset + 2`, ending at most at
    // `offset + V::BYTES + 2`. `offset` never exceeds `tail.len()`, so the
    // additions cannot overflow, and the bytes left over go to the safe
    // `scalar::dash_line`.
    unsafe {
        let newline = V::splat(b'\n');
        let dash = V::splat(b'-');
        while offset + UNROLL * V::BYTES + DASH_LOOKAHEAD <= tail.len() {
            let at = ptr.add(offset + 1);
            let dashes = [
                V::load(at).eq(dash),
                V::load(at.add(V::BYTES)).eq(dash),
                V::load(at.add(2 * V::BYTES)).eq(dash),
                V::load(at.add(3 * V::BYTES)).eq(dash),
            ];
            let [d0, d1, d2, d3] = dashes;
            if d0.or(d1).or(d2.or(d3)).mask() != 0 {
                for (index, first_dash) in dashes.into_iter().enumerate() {
                    let block = offset + index * V::BYTES;
                    let hits = V::load(ptr.add(block))
                        .eq(newline)
                        .and(first_dash)
                        .and(V::load(ptr.add(block + 2)).eq(dash));
                    let mask = hits.mask();
                    if mask != 0 {
                        return Some(block + first::<V>(mask));
                    }
                }
            }
            offset += UNROLL * V::BYTES;
        }
        while offset + V::BYTES + DASH_LOOKAHEAD <= tail.len() {
            let at = ptr.add(offset);
            let hits = V::load(at)
                .eq(newline)
                .and(V::load(at.add(1)).eq(dash))
                .and(V::load(at.add(2)).eq(dash));
            let mask = hits.mask();
            if mask != 0 {
                return Some(offset + first::<V>(mask));
            }
            offset += V::BYTES;
        }
    }
    scalar::dash_line(tail, offset)
}

/// Vector version of `scalar::field_end` over the whole of `tail`.
///
/// # Safety
///
/// The running CPU must support the instruction set of `V`.
#[inline(always)]
pub(crate) unsafe fn field_end<V: Vector>(tail: &[u8]) -> Option<usize> {
    let ptr = tail.as_ptr();
    let mut offset = 0;
    // SAFETY: our caller guarantees the CPU supports `V`. Every load reads
    // `V::BYTES` bytes that lie inside `tail`. The unrolled loop runs while
    // `offset + UNROLL * V::BYTES + FOLD_LOOKAHEAD <= tail.len()`; its loads
    // start at `offset + k * V::BYTES` for `k < UNROLL` and at `block + 1`
    // with `block <= offset + 3 * V::BYTES`, so they end at most at
    // `offset + 4 * V::BYTES + 1`, within that bound. The single-vector loop
    // runs while `offset + V::BYTES + FOLD_LOOKAHEAD <= tail.len()` and loads
    // from `offset` and `offset + 1`, ending at most at
    // `offset + V::BYTES + 1`. `offset` never exceeds `tail.len()`, so the
    // additions cannot overflow, and the bytes left over go to the safe
    // `scalar::field_end`.
    unsafe {
        let newline = V::splat(b'\n');
        let space = V::splat(b' ');
        let tab = V::splat(b'\t');
        while offset + UNROLL * V::BYTES + FOLD_LOOKAHEAD <= tail.len() {
            let at = ptr.add(offset);
            let lines = [
                V::load(at).eq(newline),
                V::load(at.add(V::BYTES)).eq(newline),
                V::load(at.add(2 * V::BYTES)).eq(newline),
                V::load(at.add(3 * V::BYTES)).eq(newline),
            ];
            let [l0, l1, l2, l3] = lines;
            if l0.or(l1).or(l2.or(l3)).mask() != 0 {
                for (index, lines) in lines.into_iter().enumerate() {
                    let block = offset + index * V::BYTES;
                    if lines.mask() == 0 {
                        continue;
                    }
                    let next = V::load(ptr.add(block + 1));
                    let mask = lines.and_not(next.eq(space).or(next.eq(tab))).mask();
                    if mask != 0 {
                        return Some(block + first::<V>(mask));
                    }
                }
            }
            offset += UNROLL * V::BYTES;
        }
        while offset + V::BYTES + FOLD_LOOKAHEAD <= tail.len() {
            let at = ptr.add(offset);
            let lines = V::load(at).eq(newline);
            if lines.mask() != 0 {
                let next = V::load(at.add(1));
                let mask = lines.and_not(next.eq(space).or(next.eq(tab))).mask();
                if mask != 0 {
                    return Some(offset + first::<V>(mask));
                }
            }
            offset += V::BYTES;
        }
    }
    scalar::field_end(tail, offset)
}

impl ByteSet {
    /// First byte of the set in `hay[from..end]`, testing `SET_WIDTH` bytes per
    /// call of `mask_at`.
    ///
    /// # Safety
    ///
    /// `mask_at` must read at most `SET_WIDTH` bytes from the pointer it gets
    /// and must otherwise be sound to call on the running CPU: this function
    /// passes it only pointers with `SET_WIDTH` bytes of `hay` behind them.
    #[inline(always)]
    pub(super) unsafe fn simd_first_in(
        &self,
        hay: &[u8],
        from: usize,
        end: usize,
        bits_per_byte: u32,
        mask_at: impl Fn(*const u8) -> u64,
    ) -> Option<usize> {
        let end = end.min(hay.len());
        if from >= end {
            return None;
        }
        let ptr = hay.as_ptr();
        let first = |offset: usize, mask: u64| {
            let hit = offset + (mask.trailing_zeros() / bits_per_byte) as usize;
            (hit < end).then_some(hit)
        };
        let mut offset = from;
        while offset + SET_WIDTH <= hay.len() {
            // SAFETY: the loop condition keeps `offset + SET_WIDTH` within
            // `hay.len()`, so `ptr + offset` and the `SET_WIDTH` bytes that
            // `mask_at` reads from it lie inside `hay`. `offset` starts below
            // `end <= hay.len()` and the loop returns once it reaches `end`, so
            // the addition cannot overflow.
            let mask = mask_at(unsafe { ptr.add(offset) });
            if mask != 0 {
                return first(offset, mask);
            }
            offset += SET_WIDTH;
            if offset >= end {
                return None;
            }
        }
        match hay.len().checked_sub(SET_WIDTH) {
            Some(base) => {
                let skipped = (offset - base) as u32 * bits_per_byte;
                // SAFETY: `base` is `hay.len() - SET_WIDTH` and the subtraction
                // did not underflow, so the `SET_WIDTH` bytes at `ptr + base` are
                // the last ones of `hay`. The loop exited with
                // `offset + SET_WIDTH > hay.len()`, so `offset > base` and the
                // shift drops the bytes before `offset`.
                let mask = mask_at(unsafe { ptr.add(base) }) >> skipped;
                if mask != 0 { first(offset, mask) } else { None }
            }
            None => self.first_in(hay, offset, end),
        }
    }

    /// First byte of the stop set in `hay[from..end]`, and whether a marked byte
    /// comes before it, testing `SET_WIDTH` bytes per call of `masks_at`.
    ///
    /// # Safety
    ///
    /// `masks_at` must read at most `SET_WIDTH` bytes from the pointer it gets
    /// and must otherwise be sound to call on the running CPU: this function
    /// passes it only pointers with `SET_WIDTH` bytes of `hay` behind them.
    #[inline(always)]
    pub(super) unsafe fn simd_stop_in(
        &self,
        hay: &[u8],
        from: usize,
        end: usize,
        bits_per_byte: u32,
        masks_at: impl Fn(*const u8) -> (u64, u64),
    ) -> Stop {
        let end = end.min(hay.len());
        let mut marked = false;
        let mut offset = from;
        while offset < end {
            let base = if offset + SET_WIDTH <= hay.len() {
                offset
            } else {
                match hay.len().checked_sub(SET_WIDTH) {
                    Some(base) => base,
                    None => {
                        let rest = self.stop_in(hay, offset, end);
                        return Stop {
                            at: rest.at,
                            marked: marked || rest.marked,
                        };
                    }
                }
            };
            let skipped = (offset - base) as u32 * bits_per_byte;
            // SAFETY: `base` is `offset` when `offset + SET_WIDTH <= hay.len()`,
            // else `hay.len() - SET_WIDTH` when that does not underflow (a
            // shorter `hay` returned through the safe `ByteSet::stop_in` above),
            // so the `SET_WIDTH` bytes that `masks_at` reads from
            // `hay.as_ptr() + base` lie inside `hay`. `offset < end <= hay.len()`
            // keeps `offset + SET_WIDTH` from overflowing.
            let (stops, marks) = masks_at(unsafe { hay.as_ptr().add(base) });
            let valid = low_bits(end - offset, bits_per_byte);
            let stops = (stops >> skipped) & valid;
            let marks = (marks >> skipped) & valid;
            if stops != 0 {
                let first = stops.trailing_zeros() / bits_per_byte;
                return Stop {
                    at: Some(offset + first as usize),
                    marked: marked || marks & low_bits(first as usize, bits_per_byte) != 0,
                };
            }
            marked |= marks != 0;
            offset = base + SET_WIDTH;
        }
        Stop { at: None, marked }
    }
}

#[inline(always)]
fn low_bits(bytes: usize, bits_per_byte: u32) -> u64 {
    match u32::try_from(bytes).map(|bytes| bytes.saturating_mul(bits_per_byte)) {
        Ok(bits) if bits < u64::BITS => (1 << bits) - 1,
        _ => u64::MAX,
    }
}
