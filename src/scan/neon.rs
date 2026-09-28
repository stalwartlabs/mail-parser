/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(unsafe_code)]

use super::{
    set::{ByteSet, Stop},
    simd::{self, Vector},
};
use core::arch::aarch64::*;

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
        // SAFETY: NEON is enabled at compile time (the module is built only with
        // `target_feature = "neon"`). `self.low` and `self.high` are `[u8; 16]`, so
        // their loads read exactly their 16 bytes. `ByteSet::simd_first_in` passes the
        // closure only pointers with `SET_WIDTH` (16) bytes of `hay` behind them,
        // and the closure reads those 16 bytes with one `vld1q_u8`.
        unsafe {
            let low = vld1q_u8(self.low.as_ptr());
            let high = vld1q_u8(self.high.as_ptr());
            let nibble = vdupq_n_u8(0x0f);
            let stops = vdupq_n_u8(self.stops);
            self.simd_first_in(hay, from, end, uint8x16_t::BITS_PER_BYTE, |at| {
                let bytes = vld1q_u8(at);
                let buckets = vandq_u8(
                    vqtbl1q_u8(low, vandq_u8(bytes, nibble)),
                    vqtbl1q_u8(high, vshrq_n_u8::<4>(bytes)),
                );
                vtstq_u8(buckets, stops).mask()
            })
        }
    }

    #[inline]
    pub(super) fn neon_stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: NEON is enabled at compile time (the module is built only with
        // `target_feature = "neon"`). `self.low` and `self.high` are `[u8; 16]`, so
        // their loads read exactly their 16 bytes. `ByteSet::simd_stop_in` passes the
        // closure only pointers with `SET_WIDTH` (16) bytes of `hay` behind them,
        // and the closure reads those 16 bytes with one `vld1q_u8`.
        unsafe {
            let low = vld1q_u8(self.low.as_ptr());
            let high = vld1q_u8(self.high.as_ptr());
            let nibble = vdupq_n_u8(0x0f);
            let stops = vdupq_n_u8(self.stops);
            let marks = vdupq_n_u8(self.marks);
            self.simd_stop_in(hay, from, end, uint8x16_t::BITS_PER_BYTE, |at| {
                let bytes = vld1q_u8(at);
                let buckets = vandq_u8(
                    vqtbl1q_u8(low, vandq_u8(bytes, nibble)),
                    vqtbl1q_u8(high, vshrq_n_u8::<4>(bytes)),
                );
                (
                    vtstq_u8(buckets, stops).mask(),
                    vtstq_u8(buckets, marks).mask(),
                )
            })
        }
    }
}
