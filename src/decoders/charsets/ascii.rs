/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use simdutf8::basic::from_utf8;

const BLOCK: usize = 32;
const WORD: usize = 8;
const HIGH_BITS: u64 = 0x8080_8080_8080_8080;

pub(super) fn split_ascii(bytes: &[u8]) -> (&str, &[u8]) {
    let (ascii, rest) = bytes.split_at(ascii_len(bytes));
    (as_ascii_str(ascii), rest)
}

pub(super) fn as_ascii_str(ascii: &[u8]) -> &str {
    from_utf8(ascii).unwrap_or_default()
}

pub(super) fn ascii_len(bytes: &[u8]) -> usize {
    let (blocks, _) = bytes.as_chunks::<BLOCK>();
    let offset = blocks
        .iter()
        .take_while(|block| is_ascii_block(block))
        .count()
        * BLOCK;
    let tail = bytes.get(offset..).unwrap_or_default();
    let (words, rest) = tail.as_chunks::<WORD>();
    for (index, word) in words.iter().enumerate() {
        let high = u64::from_le_bytes(*word) & HIGH_BITS;
        if high != 0 {
            return offset + index * WORD + (high.trailing_zeros() / 8) as usize;
        }
    }
    offset + words.len() * WORD + rest.iter().take_while(|byte| byte.is_ascii()).count()
}

fn is_ascii_block(block: &[u8; BLOCK]) -> bool {
    block.iter().fold(0, |acc, byte| acc | byte).is_ascii()
}

pub(super) fn non_ascii_count(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| !byte.is_ascii()).count()
}
