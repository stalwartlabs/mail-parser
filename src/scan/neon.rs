/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(unsafe_code)]

use super::{
    set::{ByteSet, Stop},
    simd::{self, Table, Vector},
};
use core::arch::aarch64::*;

const NIBBLE: u8 = 0x0f;

impl Vector for uint8x16_t {
    const BYTES: usize = 16;
    const BITS_PER_BYTE: u32 = 4;

    #[inline(always)]
    unsafe fn load(ptr: *const u8) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // Our caller guarantees `ptr` is readable for 16 bytes.
        unsafe { vld1q_u8(ptr) }
    }

    #[inline(always)]
    unsafe fn splat(byte: u8) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { vdupq_n_u8(byte) }
    }

    #[inline(always)]
    unsafe fn eq(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { vceqq_u8(self, other) }
    }

    #[inline(always)]
    unsafe fn and(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { vandq_u8(self, other) }
    }

    #[inline(always)]
    unsafe fn or(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { vorrq_u8(self, other) }
    }

    #[inline(always)]
    unsafe fn and_not(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { vbicq_u8(self, other) }
    }

    #[inline(always)]
    unsafe fn mask(self) -> u64 {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // The intrinsics work on registers only.
        unsafe {
            vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(vreinterpretq_u16_u8(
                self,
            ))))
        }
    }
}

impl Table for uint8x16_t {
    #[inline(always)]
    unsafe fn table(entries: &[u8; 16]) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time.
        // `entries` is 16 bytes, exactly what the load reads.
        unsafe { vld1q_u8(entries.as_ptr()) }
    }

    #[inline(always)]
    unsafe fn lookup_low(self, table: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time. The
        // intrinsics work on registers only, and the mask keeps every index
        // below 16, so no lane selects outside `table`.
        unsafe { vqtbl1q_u8(table, vandq_u8(self, vdupq_n_u8(NIBBLE))) }
    }

    #[inline(always)]
    unsafe fn lookup_high(self, table: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time. The
        // intrinsics work on registers only, and shifting a byte right by four
        // leaves every index below 16, so no lane selects outside `table`.
        unsafe { vqtbl1q_u8(table, vshrq_n_u8::<4>(self)) }
    }

    #[inline(always)]
    unsafe fn test_mask(self, mask: Self) -> u64 {
        // SAFETY: the module is built only with `target_feature = "neon"`, so
        // NEON is part of the compilation target and present at run time. The
        // intrinsics work on registers only.
        unsafe { vtstq_u8(self, mask).mask() }
    }
}

#[inline]
pub(crate) fn dash_line(hay: &[u8], from: usize) -> Option<usize> {
    // SAFETY: NEON is enabled at compile time (the module is built only
    // with `target_feature = "neon"`), the only requirement of
    // `simd::dash_line::<uint8x16_t>`.
    unsafe { simd::dash_line::<uint8x16_t>(hay.get(from..)?) }.map(|pos| pos + from)
}

#[inline]
pub(crate) fn field_end(hay: &[u8], from: usize) -> Option<usize> {
    // SAFETY: NEON is enabled at compile time (the module is built only
    // with `target_feature = "neon"`), the only requirement of
    // `simd::field_end::<uint8x16_t>`.
    unsafe { simd::field_end::<uint8x16_t>(hay.get(from..)?) }.map(|pos| pos + from)
}

impl ByteSet {
    #[inline]
    pub(super) fn neon_first_in(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: NEON is enabled at compile time (the module is built only
        // with `target_feature = "neon"`), which is what `uint8x16_t` needs as
        // both a `Vector` and a `Table`. `self.low` and `self.high` are
        // `[u8; 16]`, the size `Table::table` reads, and the closure reads the
        // `V::BYTES` bytes that `simd_first_in` guarantees behind the pointer it
        // gets.
        unsafe {
            let low = uint8x16_t::table(&self.low);
            let high = uint8x16_t::table(&self.high);
            let stops = uint8x16_t::splat(self.stops);
            self.simd_first_in::<uint8x16_t>(hay, from, end, |at| {
                simd::buckets::<uint8x16_t>(at, low, high).test_mask(stops)
            })
        }
    }

    #[inline]
    pub(super) fn neon_stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: NEON is enabled at compile time (the module is built only
        // with `target_feature = "neon"`), which is what `uint8x16_t` needs as
        // both a `Vector` and a `Table`. `self.low` and `self.high` are
        // `[u8; 16]`, the size `Table::table` reads, and the closure reads the
        // `V::BYTES` bytes that `simd_stop_in` guarantees behind the pointer it
        // gets.
        unsafe {
            let low = uint8x16_t::table(&self.low);
            let high = uint8x16_t::table(&self.high);
            let stops = uint8x16_t::splat(self.stops);
            let marks = uint8x16_t::splat(self.marks);
            self.simd_stop_in::<uint8x16_t>(hay, from, end, |at| {
                let buckets = simd::buckets::<uint8x16_t>(at, low, high);
                (buckets.test_mask(stops), buckets.test_mask(marks))
            })
        }
    }
}
