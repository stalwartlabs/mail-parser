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
use core::arch::x86_64::*;

const NIBBLE: i8 = 0x0f;

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

impl Table for __m128i {
    #[inline(always)]
    unsafe fn table(entries: &[u8; 16]) -> Self {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time.
        // `entries` is 16 bytes, exactly what the unaligned load reads.
        unsafe { _mm_loadu_si128(entries.as_ptr().cast()) }
    }

    #[inline(always)]
    unsafe fn lookup_low(self, table: Self) -> Self {
        // SAFETY: our caller guarantees the CPU supports SSSE3, which
        // `_mm_shuffle_epi8` needs; the other intrinsics need only SSE2. All
        // three work on registers only, and the mask keeps every shuffle index
        // below 16, so no lane selects outside `table`.
        unsafe { _mm_shuffle_epi8(table, _mm_and_si128(self, _mm_set1_epi8(NIBBLE))) }
    }

    #[inline(always)]
    unsafe fn lookup_high(self, table: Self) -> Self {
        // SAFETY: our caller guarantees the CPU supports SSSE3, which
        // `_mm_shuffle_epi8` needs; the other intrinsics need only SSE2. All
        // four work on registers only, and the mask keeps every shuffle index
        // below 16, so no lane selects outside `table`.
        unsafe {
            _mm_shuffle_epi8(
                table,
                _mm_and_si128(_mm_srli_epi16::<4>(self), _mm_set1_epi8(NIBBLE)),
            )
        }
    }

    #[inline(always)]
    unsafe fn test_mask(self, mask: Self) -> u64 {
        // SAFETY: the module is built only with `target_feature = "sse2"`, so
        // SSE2 is part of the compilation target and present at run time. The
        // intrinsics work on registers only.
        unsafe {
            let hits = _mm_and_si128(self, mask);
            let zeros = _mm_movemask_epi8(_mm_cmpeq_epi8(hits, _mm_setzero_si128()));
            u64::from(!(zeros as u32) & 0xffff)
        }
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
        // pointers with `__m128i::BYTES` bytes of `hay` behind them, and
        // `__m128i::load` reads those bytes.
        unsafe {
            self.simd_first_in::<__m128i>(hay, from, end, |at| {
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
    /// The running CPU must support SSSE3.
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_first_in_impl(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: our caller guarantees the CPU supports SSSE3 and this
        // function enables it, which covers what `__m128i` needs: SSE2 as a `Vector` and SSSE3 as a `Table`.
        // The closure is defined here so that it inherits the enabled feature,
        // and reads the `V::BYTES` bytes that `simd_first_in` guarantees behind the
        // pointer it gets.
        unsafe {
            let low = __m128i::table(&self.low);
            let high = __m128i::table(&self.high);
            let stops = __m128i::splat(self.stops);
            self.simd_first_in::<__m128i>(hay, from, end, |at| {
                simd::buckets::<__m128i>(at, low, high).test_mask(stops)
            })
        }
    }

    #[inline]
    pub(super) fn ssse3_first_in(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: the CPU supports SSSE3. This method is reached only from
        // the `Kernel` dispatch in `scan/mod.rs` on `Backend::Ssse3`: it is
        // `pub(super)` in `x86`, a private module of `scan`, `Backend` and the
        // field of `Kernel` are private to `scan`, and `Kernel::best` and
        // `Kernel::available` build that back end only after
        // `is_x86_feature_detected!("ssse3")` returned true.
        unsafe { self.ssse3_first_in_impl(hay, from, end) }
    }

    /// # Safety
    ///
    /// The running CPU must support AVX2.
    #[target_feature(enable = "avx2")]
    unsafe fn avx2_first_in_impl(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: our caller guarantees the CPU supports AVX2 and this
        // function enables it, which implies both instruction sets `__m128i`
        // needs: SSE2 as a `Vector` and SSSE3 as a `Table`. Enabling AVX2 keeps
        // these 128-bit operations VEX-encoded, so they do not pay an AVX to SSE
        // transition. The closure is defined here so that it inherits the
        // enabled feature, and reads the `V::BYTES` bytes that `simd_first_in`
        // guarantees behind the pointer it gets.
        unsafe {
            let low = __m128i::table(&self.low);
            let high = __m128i::table(&self.high);
            let stops = __m128i::splat(self.stops);
            self.simd_first_in::<__m128i>(hay, from, end, |at| {
                simd::buckets::<__m128i>(at, low, high).test_mask(stops)
            })
        }
    }

    #[inline]
    pub(super) fn avx2_first_in(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        // SAFETY: the CPU supports AVX2. This method is reached only from
        // the `Kernel` dispatch in `scan/mod.rs` on `Backend::Avx2`: it is
        // `pub(super)` in `x86`, a private module of `scan`, `Backend` and the
        // field of `Kernel` are private to `scan`, and `Kernel::best` and
        // `Kernel::available` build that back end only after
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
        // pointers with `__m128i::BYTES` bytes of `hay` behind them, and
        // `__m128i::load` reads those bytes.
        unsafe {
            self.simd_stop_in::<__m128i>(hay, from, end, |at| {
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
    /// The running CPU must support SSSE3.
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_stop_in_impl(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: our caller guarantees the CPU supports SSSE3 and this
        // function enables it, which covers what `__m128i` needs: SSE2 as a `Vector` and SSSE3 as a `Table`.
        // The closure is defined here so that it inherits the enabled feature,
        // and reads the `V::BYTES` bytes that `simd_stop_in` guarantees behind the
        // pointer it gets.
        unsafe {
            let low = __m128i::table(&self.low);
            let high = __m128i::table(&self.high);
            let stops = __m128i::splat(self.stops);
            let marks = __m128i::splat(self.marks);
            self.simd_stop_in::<__m128i>(hay, from, end, |at| {
                let buckets = simd::buckets::<__m128i>(at, low, high);
                (buckets.test_mask(stops), buckets.test_mask(marks))
            })
        }
    }

    #[inline]
    pub(super) fn ssse3_stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: the CPU supports SSSE3. This method is reached only from
        // the `Kernel` dispatch in `scan/mod.rs` on `Backend::Ssse3`: it is
        // `pub(super)` in `x86`, a private module of `scan`, `Backend` and the
        // field of `Kernel` are private to `scan`, and `Kernel::best` and
        // `Kernel::available` build that back end only after
        // `is_x86_feature_detected!("ssse3")` returned true.
        unsafe { self.ssse3_stop_in_impl(hay, from, end) }
    }

    /// # Safety
    ///
    /// The running CPU must support AVX2.
    #[target_feature(enable = "avx2")]
    unsafe fn avx2_stop_in_impl(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: our caller guarantees the CPU supports AVX2 and this
        // function enables it, which implies both instruction sets `__m128i`
        // needs: SSE2 as a `Vector` and SSSE3 as a `Table`. Enabling AVX2 keeps
        // these 128-bit operations VEX-encoded, so they do not pay an AVX to SSE
        // transition. The closure is defined here so that it inherits the
        // enabled feature, and reads the `V::BYTES` bytes that `simd_stop_in`
        // guarantees behind the pointer it gets.
        unsafe {
            let low = __m128i::table(&self.low);
            let high = __m128i::table(&self.high);
            let stops = __m128i::splat(self.stops);
            let marks = __m128i::splat(self.marks);
            self.simd_stop_in::<__m128i>(hay, from, end, |at| {
                let buckets = simd::buckets::<__m128i>(at, low, high);
                (buckets.test_mask(stops), buckets.test_mask(marks))
            })
        }
    }

    #[inline]
    pub(super) fn avx2_stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        // SAFETY: the CPU supports AVX2. This method is reached only from
        // the `Kernel` dispatch in `scan/mod.rs` on `Backend::Avx2`: it is
        // `pub(super)` in `x86`, a private module of `scan`, `Backend` and the
        // field of `Kernel` are private to `scan`, and `Kernel::best` and
        // `Kernel::available` build that back end only after
        // `is_x86_feature_detected!("avx2")` returned true.
        unsafe { self.avx2_stop_in_impl(hay, from, end) }
    }
}
