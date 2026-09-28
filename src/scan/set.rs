/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

const NIBBLE: u8 = 0x0f;
const MAX_MEMBERS: usize = 16;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ByteSet {
    pub(super) low: [u8; 16],
    pub(super) high: [u8; 16],
    pub(super) stops: u8,
    pub(super) marks: u8,
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    members: [u8; MAX_MEMBERS],
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    len: usize,
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    marked: [u8; MAX_MEMBERS],
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    marked_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stop {
    pub(crate) at: Option<usize>,
    pub(crate) marked: bool,
}

impl ByteSet {
    pub(crate) const fn new(bytes: &[u8]) -> ByteSet {
        ByteSet::with_marks(bytes, &[])
    }

    pub(crate) const fn with_marks(bytes: &[u8], marks: &[u8]) -> ByteSet {
        assert!(!bytes.is_empty() && bytes.len() <= MAX_MEMBERS && marks.len() <= MAX_MEMBERS);
        let mut set = ByteSet {
            low: [0; 16],
            high: [0; 16],
            stops: 0,
            marks: 0,
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            members: [0; MAX_MEMBERS],
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            len: bytes.len(),
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            marked: [0; MAX_MEMBERS],
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            marked_len: marks.len(),
        };
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            assert!(byte < 0x80);
            let bucket = 1 << (byte >> 4);
            set.low[(byte & NIBBLE) as usize] |= bucket;
            set.high[(byte >> 4) as usize] = bucket;
            set.stops |= bucket;
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            {
                set.members[index] = byte;
            }
            index += 1;
        }
        index = 0;
        while index < marks.len() {
            let byte = marks[index];
            assert!(byte < 0x80);
            let bucket = 1 << (byte >> 4);
            assert!(set.stops & bucket == 0);
            set.low[(byte & NIBBLE) as usize] |= bucket;
            set.high[(byte >> 4) as usize] = bucket;
            set.marks |= bucket;
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            {
                set.marked[index] = byte;
            }
            index += 1;
        }
        set
    }

    pub(crate) const fn excluding(classes: &[u8; 256], class: u8) -> ByteSet {
        let mut set = ByteSet {
            low: [0; 16],
            high: [0; 16],
            stops: 0,
            marks: 0,
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            members: [0; MAX_MEMBERS],
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            len: 0,
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            marked: [0; MAX_MEMBERS],
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            marked_len: 0,
        };
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        let mut count = 0;
        let mut byte = 0;
        while byte < classes.len() {
            let member = classes[byte] & class == 0;
            if byte >= 0x80 {
                assert!(!member);
            } else if member {
                let value = byte as u8;
                let bucket = 1 << (value >> 4);
                set.low[(value & NIBBLE) as usize] |= bucket;
                set.high[(value >> 4) as usize] = bucket;
                set.stops |= bucket;
                #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
                {
                    if count < MAX_MEMBERS {
                        set.members[count] = value;
                    }
                    count += 1;
                }
            }
            byte += 1;
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        {
            set.len = if count <= MAX_MEMBERS { count } else { 0 };
        }
        set
    }

    #[inline(always)]
    fn buckets(&self, byte: u8) -> u8 {
        let low = self.low.get(usize::from(byte & NIBBLE)).copied();
        let high = self.high.get(usize::from(byte >> 4)).copied();
        low.unwrap_or(0) & high.unwrap_or(0)
    }

    #[inline(always)]
    pub(crate) fn contains(&self, byte: u8) -> bool {
        self.buckets(byte) & self.stops != 0
    }

    #[inline(always)]
    pub(crate) fn marks(&self, byte: u8) -> bool {
        self.buckets(byte) & self.marks != 0
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    pub(super) fn members(&self) -> &[u8] {
        self.members.get(..self.len).unwrap_or_default()
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    pub(super) fn marked(&self) -> &[u8] {
        self.marked.get(..self.marked_len).unwrap_or_default()
    }

    pub(crate) fn first_in(&self, hay: &[u8], from: usize, end: usize) -> Option<usize> {
        hay.get(from..end.min(hay.len()))?
            .iter()
            .position(|&byte| self.contains(byte))
            .map(|pos| pos + from)
    }

    pub(crate) fn stop_in(&self, hay: &[u8], from: usize, end: usize) -> Stop {
        let mut marked = false;
        let at = hay
            .get(from..end.min(hay.len()))
            .unwrap_or_default()
            .iter()
            .position(|&byte| {
                marked |= self.marks(byte);
                self.contains(byte)
            })
            .map(|pos| pos + from);
        Stop { at, marked }
    }
}
