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
use core::arch::x86_64::*;

impl Vector for __m128i {
    const BYTES: usize = 16;
    const BITS_PER_BYTE: u32 = 1;

    #[inline(always)]
    unsafe fn load(ptr: *const u8) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // Our caller guarantees `ptr` is readable for 16 bytes; the load is
        // unaligned.
        unsafe { _mm_loadu_si128(ptr.cast()) }
    }

    #[inline(always)]
    unsafe fn splat(byte: u8) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { _mm_set1_epi8(byte as i8) }
    }

    #[inline(always)]
    unsafe fn eq(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { _mm_cmpeq_epi8(self, other) }
    }

    #[inline(always)]
    unsafe fn and(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { _mm_and_si128(self, other) }
    }

    #[inline(always)]
    unsafe fn or(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { _mm_or_si128(self, other) }
    }

    #[inline(always)]
    unsafe fn and_not(self, other: Self) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { _mm_andnot_si128(other, self) }
    }

    #[inline(always)]
    unsafe fn mask(self) -> u64 {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // The intrinsic works on registers only.
        unsafe { _mm_movemask_epi8(self) as u32 as u64 }
    }
}

impl Vector for __m256i {
    const BYTES: usize = 32;
    const BITS_PER_BYTE: u32 = 1;

    #[inline(always)]
    unsafe fn load(ptr: *const u8) -> Self {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // It also guarantees `ptr` is readable for 32 bytes; the load is
        // unaligned.
        unsafe { _mm256_loadu_si256(ptr.cast()) }
    }

    #[inline(always)]
    unsafe fn splat(byte: u8) -> Self {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // The intrinsic works on registers only.
        unsafe { _mm256_set1_epi8(byte as i8) }
    }

    #[inline(always)]
    unsafe fn eq(self, other: Self) -> Self {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // The intrinsic works on registers only.
        unsafe { _mm256_cmpeq_epi8(self, other) }
    }

    #[inline(always)]
    unsafe fn and(self, other: Self) -> Self {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // The intrinsic works on registers only.
        unsafe { _mm256_and_si256(self, other) }
    }

    #[inline(always)]
    unsafe fn or(self, other: Self) -> Self {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // The intrinsic works on registers only.
        unsafe { _mm256_or_si256(self, other) }
    }

    #[inline(always)]
    unsafe fn and_not(self, other: Self) -> Self {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // The intrinsic works on registers only.
        unsafe { _mm256_andnot_si256(other, self) }
    }

    #[inline(always)]
    unsafe fn mask(self) -> u64 {
        // SAFETY: our caller guarantees the CPU supports AVX2, as the
        // `Vector` contract requires for `__m256i`.
        // The intrinsic works on registers only.
        unsafe { _mm256_movemask_epi8(self) as u32 as u64 }
    }
}

#[inline]
pub(crate) fn sse2_dash_line(hay: &[u8], from: usize) -> Option<usize> {
    // SAFETY: the module is built only with `target_feature = "sse2"`, so
    // SSE2 is present, the only requirement of `simd::dash_line::<__m128i>`.
    unsafe { simd::dash_line::<__m128i>(hay.get(from..)?) }.map(|pos| pos + from)
}

#[inline]
pub(crate) fn sse2_field_end(hay: &[u8], from: usize) -> Option<usize> {
    // SAFETY: the module is built only with `target_feature = "sse2"`, so
    // SSE2 is present, the only requirement of `simd::field_end::<__m128i>`.
    unsafe { simd::field_end::<__m128i>(hay.get(from..)?) }.map(|pos| pos + from)
}

/// # Safety
///
/// The running CPU must support AVX2.
#[target_feature(enable = "avx2")]
unsafe fn avx2_dash_line_impl(tail: &[u8]) -> Option<usize> {
    // SAFETY: our caller guarantees the CPU supports AVX2, the only
    // requirement of `simd::dash_line::<__m256i>`.
    unsafe { simd::dash_line::<__m256i>(tail) }
}

/// # Safety
///
/// The running CPU must support AVX2.
#[target_feature(enable = "avx2")]
unsafe fn avx2_field_end_impl(tail: &[u8]) -> Option<usize> {
    // SAFETY: our caller guarantees the CPU supports AVX2, the only
    // requirement of `simd::field_end::<__m256i>`.
    unsafe { simd::field_end::<__m256i>(tail) }
}

#[inline]
pub(crate) fn avx2_dash_line(hay: &[u8], from: usize) -> Option<usize> {
    // SAFETY: the CPU supports AVX2. This function is reached only from the
    // `Kernel` dispatch in `scan/mod.rs` on `Backend::Avx2`: `x86` is a
    // private module of `scan`, `Backend` and the field of `Kernel` are
    // private to `scan`, and `Kernel::best` and `Kernel::available` build
    // `Backend::Avx2` only after `is_x86_feature_detected!("avx2")` returned
    // true.
    unsafe { avx2_dash_line_impl(hay.get(from..)?) }.map(|pos| pos + from)
}

#[inline]
pub(crate) fn avx2_field_end(hay: &[u8], from: usize) -> Option<usize> {
    // SAFETY: the CPU supports AVX2. This function is reached only from the
    // `Kernel` dispatch in `scan/mod.rs` on `Backend::Avx2`: `x86` is a
    // private module of `scan`, `Backend` and the field of `Kernel` are
    // private to `scan`, and `Kernel::best` and `Kernel::available` build
    // `Backend::Avx2` only after `is_x86_feature_detected!("avx2")` returned
    // true.
    unsafe { avx2_field_end_impl(hay.get(from..)?) }.map(|pos| pos + from)
}

impl ByteSet {
    #[inline]
    pub(super) fn sse2_first_in(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        if self.members().is_empty() {
            return self.first_in(hay, from, end);
        }
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is present, and the closure uses only SSE2 intrinsics and the
        // `__m128i` methods. `ByteSet::simd_first_in` passes the closure only
        // pointers with `SET_WIDTH` (16) bytes of `hay` behind them, and
        // `__m128i::load` reads those 16 bytes.
        unsafe {
            self.simd_first_in(hay, from, end, __m128i::BITS_PER_BYTE, |at| {
                let bytes = __m128i::load(at);
                self.members()
                    .iter()
                    .fold(_mm_setzero_si128(), |hits, &member| {
                        hits.or(bytes.eq(__m128i::splat(member)))
                    })
                    .mask()
            })
        }
    }

    /// # Safety
    ///
    /// The running CPU must support AVX2.
    #[target_feature(enable = "avx2")]
    unsafe fn avx2_first_in_impl(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: our caller guarantees the CPU supports AVX2, and this function
        // enables it; AVX2 implies SSSE3 (`_mm_shuffle_epi8`) and SSE2, the only
        // instruction sets used here. `self.low` and `self.high` are `[u8; 16]`, so
        // their unaligned 16-byte loads stay inside them. `ByteSet::simd_first_in`
        // passes the closure only pointers with `SET_WIDTH` (16) bytes of `hay`
        // behind them, and the closure reads those 16 bytes with one unaligned
        // load.
        unsafe {
            let low = _mm_loadu_si128(self.low.as_ptr().cast());
            let high = _mm_loadu_si128(self.high.as_ptr().cast());
            let nibble = _mm_set1_epi8(0x0f);
            let stops = _mm_set1_epi8(self.stops as i8);
            let zero = _mm_setzero_si128();
            self.simd_first_in(hay, from, end, __m128i::BITS_PER_BYTE, |at| {
                let bytes = _mm_loadu_si128(at.cast());
                let buckets = _mm_and_si128(
                    _mm_shuffle_epi8(low, _mm_and_si128(bytes, nibble)),
                    _mm_shuffle_epi8(high, _mm_and_si128(_mm_srli_epi16::<4>(bytes), nibble)),
                );
                let hits = _mm_and_si128(buckets, stops);
                u64::from(!(_mm_movemask_epi8(_mm_cmpeq_epi8(hits, zero)) as u32) & 0xffff)
            })
        }
    }

    #[inline]
    pub(super) fn avx2_first_in(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: the CPU supports AVX2. This method is reached only from the
        // `Kernel` dispatch in `scan/mod.rs` on `Backend::Avx2`: it is
        // `pub(super)` in `x86`, a private module of `scan`, `Backend` and the
        // field of `Kernel` are private to `scan`, and `Kernel::best` and
        // `Kernel::available` build `Backend::Avx2` only after
        // `is_x86_feature_detected!("avx2")` returned true.
        unsafe { self.avx2_first_in_impl(hay, from, end) }
    }

    #[inline]
    pub(super) fn sse2_stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        if self.members().is_empty() {
            return self.stop_in(hay, from, end);
        }
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is present, and the closure uses only SSE2 intrinsics and the
        // `__m128i` methods. `ByteSet::simd_stop_in` passes the closure only
        // pointers with `SET_WIDTH` (16) bytes of `hay` behind them, and
        // `__m128i::load` reads those 16 bytes.
        unsafe {
            self.simd_stop_in(hay, from, end, __m128i::BITS_PER_BYTE, |at| {
                let bytes = __m128i::load(at);
                let any = |members: &[u8]| {
                    members
                        .iter()
                        .fold(_mm_setzero_si128(), |hits, &member| {
                            hits.or(bytes.eq(__m128i::splat(member)))
                        })
                        .mask()
                };
                (any(self.members()), any(self.marked()))
            })
        }
    }

    /// # Safety
    ///
    /// The running CPU must support AVX2.
    #[target_feature(enable = "avx2")]
    unsafe fn avx2_stop_in_impl(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: our caller guarantees the CPU supports AVX2, and this function
        // enables it; AVX2 implies SSSE3 (`_mm_shuffle_epi8`) and SSE2, the only
        // instruction sets used here. `self.low` and `self.high` are `[u8; 16]`, so
        // their unaligned 16-byte loads stay inside them. `ByteSet::simd_stop_in`
        // passes the closure only pointers with `SET_WIDTH` (16) bytes of `hay`
        // behind them, and the closure reads those 16 bytes with one unaligned
        // load.
        unsafe {
            let low = _mm_loadu_si128(self.low.as_ptr().cast());
            let high = _mm_loadu_si128(self.high.as_ptr().cast());
            let nibble = _mm_set1_epi8(0x0f);
            let stops = _mm_set1_epi8(self.stops as i8);
            let marks = _mm_set1_epi8(self.marks as i8);
            let zero = _mm_setzero_si128();
            self.simd_stop_in(hay, from, end, __m128i::BITS_PER_BYTE, |at| {
                let bytes = _mm_loadu_si128(at.cast());
                let buckets = _mm_and_si128(
                    _mm_shuffle_epi8(low, _mm_and_si128(bytes, nibble)),
                    _mm_shuffle_epi8(high, _mm_and_si128(_mm_srli_epi16::<4>(bytes), nibble)),
                );
                let bits = |mask| {
                    let hits = _mm_and_si128(buckets, mask);
                    u64::from(!(_mm_movemask_epi8(_mm_cmpeq_epi8(hits, zero)) as u32) & 0xffff)
                };
                (bits(stops), bits(marks))
            })
        }
    }

    #[inline]
    pub(super) fn avx2_stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: the CPU supports AVX2. This method is reached only from the
        // `Kernel` dispatch in `scan/mod.rs` on `Backend::Avx2`: it is
        // `pub(super)` in `x86`, a private module of `scan`, `Backend` and the
        // field of `Kernel` are private to `scan`, and `Kernel::best` and
        // `Kernel::available` build `Backend::Avx2` only after
        // `is_x86_feature_detected!("avx2")` returned true.
        unsafe { self.avx2_stop_in_impl(hay, from, end) }
    }
}
