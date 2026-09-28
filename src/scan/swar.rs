/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

pub(crate) const WORD: usize = 8;
pub(crate) const LANES: u64 = 0x0101_0101_0101_0101;
pub(crate) const HIGH_BITS: u64 = 0x8080_8080_8080_8080;

#[inline(always)]
pub(crate) fn load(bytes: &[u8]) -> u64 {
    match bytes.first_chunk::<WORD>() {
        Some(word) => u64::from_le_bytes(*word),
        None => {
            let mut word = [0u8; WORD];
            if let Some(prefix) = word.get_mut(..bytes.len()) {
                prefix.copy_from_slice(bytes);
            }
            u64::from_le_bytes(word)
        }
    }
}

#[inline(always)]
pub(crate) fn splat(byte: u8) -> u64 {
    LANES * u64::from(byte)
}

#[inline(always)]
pub(crate) fn zero_bytes(word: u64) -> u64 {
    word.wrapping_sub(LANES) & !word & HIGH_BITS
}

#[inline(always)]
pub(crate) fn equal_bytes(word: u64, byte: u8) -> u64 {
    zero_bytes(word ^ splat(byte))
}

#[inline(always)]
pub(crate) fn bytes_below(word: u64, limit: u8) -> u64 {
    word.wrapping_sub(splat(limit)) & !word & HIGH_BITS
}

#[inline(always)]
pub(crate) fn ascii_in_range(word: u64, low: u8, high: u8) -> u64 {
    let low_bits = word & !HIGH_BITS;
    let from_low = low_bits + splat(0x80 - low);
    let after_high = low_bits + splat(0x7f - high);
    from_low & !after_high & !word & HIGH_BITS
}

#[inline(always)]
pub(crate) fn ascii_lowercase(word: u64) -> u64 {
    word | (ascii_in_range(word, b'A', b'Z') >> 2)
}

#[inline(always)]
pub(crate) fn first_byte(mask: u64) -> usize {
    (mask.trailing_zeros() / 8) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_match_byte_tests() {
        let mut rng = crate::scan::tests::Rng(0x0123_4567_89ab_cdef);
        for _ in 0..100_000 {
            let bytes: [u8; WORD] = std::array::from_fn(|_| rng.below(256) as u8);
            let word = u64::from_le_bytes(bytes);
            let first = |mask: u64| (mask != 0).then(|| first_byte(mask));
            assert_eq!(
                first(equal_bytes(word, b':')),
                bytes.iter().position(|&b| b == b':')
            );
            assert_eq!(
                first(bytes_below(word, 0x21)),
                bytes.iter().position(|&b| b < 0x21)
            );
            let range = ascii_in_range(word, 0x3a, 0x3d);
            for (index, byte) in bytes.iter().enumerate() {
                assert_eq!(
                    range >> (index * 8 + 7) & 1 == 1,
                    (0x3a..=0x3d).contains(byte),
                    "{bytes:?}"
                );
            }
            assert_eq!(
                ascii_lowercase(word).to_le_bytes(),
                bytes.map(|byte| byte.to_ascii_lowercase())
            );
        }
    }
}
