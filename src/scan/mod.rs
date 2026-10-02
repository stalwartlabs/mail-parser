/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Byte-scanning kernels used by the structural pass.
//!
//! Two searches do all the scanning: [`Kernel::dash_line`] finds candidate
//! delimiter lines (`\n--`) inside bodies, and [`Kernel::field_end`] finds the
//! end of each unfolded header field (a `\n` not followed by SP or HT). Every
//! back end returns exactly what the scalar reference returns.

mod lib_memchr;
mod scalar;
mod set;
pub(crate) mod swar;

pub(crate) use set::{ByteSet, Stop};

#[cfg(any(
    all(target_arch = "aarch64", target_feature = "neon"),
    all(target_arch = "x86_64", target_feature = "sse2")
))]
mod simd;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
mod neon;
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod x86;

/// A back end for the structural kernels.
///
/// A value names a back end the running CPU supports: the only ways to get
/// one are [`Kernel::best`], [`Kernel::available`] and the portable
/// [`Kernel::SCALAR`] and [`Kernel::MEMCHR`]. A SIMD back end cannot be
/// named from outside the crate:
///
/// ```compile_fail
/// #[cfg(target_arch = "x86_64")]
/// let forced = mail_parser::scan::Kernel::Avx2;
/// #[cfg(target_arch = "aarch64")]
/// let forced = mail_parser::scan::Kernel::Neon;
/// assert_eq!(forced.dash_line(b"x\n--", 0), Some(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kernel(Backend);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Backend {
    Scalar,
    Memchr,
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    Neon,
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    Sse2,
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    Ssse3,
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    Avx2,
}

impl Kernel {
    /// Byte-at-a-time reference implementation, on every CPU.
    pub const SCALAR: Kernel = Kernel(Backend::Scalar);

    /// Portable implementation over the `memchr` crate, on every CPU.
    pub const MEMCHR: Kernel = Kernel(Backend::Memchr);

    /// The fastest back end supported by the running CPU: NEON on aarch64,
    /// AVX2 or else SSSE3 when detected or else SSE2 on x86_64, `memchr`
    /// elsewhere.
    pub fn best() -> Self {
        #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
        {
            Kernel(Backend::Neon)
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        {
            if std::is_x86_feature_detected!("avx2") {
                Kernel(Backend::Avx2)
            } else if std::is_x86_feature_detected!("ssse3") {
                Kernel(Backend::Ssse3)
            } else {
                Kernel(Backend::Sse2)
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_feature = "neon"),
            all(target_arch = "x86_64", target_feature = "sse2")
        )))]
        {
            Kernel::MEMCHR
        }
    }

    /// Every back end the running CPU supports, scalar reference first.
    pub fn available() -> impl Iterator<Item = Kernel> {
        [
            Some(Kernel::SCALAR),
            Some(Kernel::MEMCHR),
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Some(Kernel(Backend::Neon)),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Some(Kernel(Backend::Sse2)),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            std::is_x86_feature_detected!("ssse3").then_some(Kernel(Backend::Ssse3)),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            std::is_x86_feature_detected!("avx2").then_some(Kernel(Backend::Avx2)),
        ]
        .into_iter()
        .flatten()
    }

    /// Short name of the back end.
    pub fn name(self) -> &'static str {
        match self.0 {
            Backend::Scalar => "scalar",
            Backend::Memchr => "memchr",
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Backend::Neon => "neon",
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Sse2 => "sse2",
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Ssse3 => "ssse3",
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Avx2 => "avx2",
        }
    }

    /// First position `i >= from` where `hay[i] == b'\n'` and
    /// `hay[i + 1..i + 3] == b"--"`.
    #[inline]
    pub fn dash_line(self, hay: &[u8], from: usize) -> Option<usize> {
        match self.0 {
            Backend::Scalar => scalar::dash_line(hay, from),
            Backend::Memchr => lib_memchr::dash_line(hay, from),
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Backend::Neon => neon::dash_line(hay, from),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Sse2 | Backend::Ssse3 => x86::sse2_dash_line(hay, from),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Avx2 => x86::avx2_dash_line(hay, from),
        }
    }

    /// First position `i >= from` where `hay[i] == b'\n'` and the next byte
    /// is not SP or HT (or `i` is the last byte).
    #[inline]
    pub fn field_end(self, hay: &[u8], from: usize) -> Option<usize> {
        match self.0 {
            Backend::Scalar => scalar::field_end(hay, from),
            Backend::Memchr => lib_memchr::field_end(hay, from),
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Backend::Neon => neon::field_end(hay, from),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Sse2 | Backend::Ssse3 => x86::sse2_field_end(hay, from),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Avx2 => x86::avx2_field_end(hay, from),
        }
    }

    #[inline]
    pub(crate) fn first_in_set(
        self,
        set: &ByteSet,
        hay: &[u8],
        from: usize,
        end: usize,
    ) -> Option<usize> {
        match self.0 {
            Backend::Scalar | Backend::Memchr => set.first_in(hay, from, end),
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Backend::Neon => set.neon_first_in(hay, from, end),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Sse2 => set.sse2_first_in(hay, from, end),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Ssse3 => set.ssse3_first_in(hay, from, end),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Avx2 => set.avx2_first_in(hay, from, end),
        }
    }

    #[inline]
    pub(crate) fn stop_in_set(self, set: &ByteSet, hay: &[u8], from: usize, end: usize) -> Stop {
        match self.0 {
            Backend::Scalar | Backend::Memchr => set.stop_in(hay, from, end),
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Backend::Neon => set.neon_stop_in(hay, from, end),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Sse2 => set.sse2_stop_in(hay, from, end),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Ssse3 => set.ssse3_stop_in(hay, from, end),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            Backend::Avx2 => set.avx2_stop_in(hay, from, end),
        }
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Kernel::best()
    }
}

#[inline(always)]
pub(crate) fn is_field_end(after: &[u8]) -> bool {
    !matches!(after, [b' ' | b'\t', ..])
}

#[inline(always)]
pub(crate) fn after(hay: &[u8], pos: usize) -> &[u8] {
    hay.get(pos + 1..).unwrap_or_default()
}

#[cfg(test)]
pub(crate) mod tests;
