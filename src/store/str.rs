/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::ops::Range;

const POOL_BIT: u32 = 1 << 31;
const MAX_LEN: usize = (POOL_BIT - 1) as usize;
const NONE_LEN: u32 = u32::MAX;
const FIRST_POOL: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Str {
    start: u32,
    len: u32,
}

impl super::Blank for Str {
    const BLANK: Str = Str::NONE;
}

impl Default for Str {
    fn default() -> Self {
        Str::EMPTY
    }
}

impl Str {
    pub(crate) const EMPTY: Str = Str {
        start: 0,
        len: POOL_BIT,
    };
    pub(crate) const NONE: Str = Str {
        start: u32::MAX,
        len: NONE_LEN,
    };

    pub(crate) fn is_none(self) -> bool {
        self.len == NONE_LEN
    }

    pub(crate) fn is_some(self) -> bool {
        !self.is_none()
    }

    pub(crate) fn from_option(value: Option<Str>) -> Str {
        value.unwrap_or(Str::NONE)
    }

    pub(crate) fn len(self) -> usize {
        if self.is_none() {
            0
        } else {
            (self.len & !POOL_BIT) as usize
        }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.len() == 0
    }

    fn is_pool(self) -> bool {
        self.len & POOL_BIT != 0
    }

    fn range(self) -> Range<usize> {
        let start = self.start as usize;
        start..start + self.len()
    }

    #[inline]
    pub(crate) fn borrow(source: &[u8], range: Range<usize>) -> Option<Str> {
        let bytes = source.get(range.clone())?;
        let len = bytes.len();
        if len > MAX_LEN || range.start > u32::MAX as usize || !bytes.is_ascii() && !is_utf8(bytes)
        {
            return None;
        }
        Some(Str {
            start: range.start as u32,
            len: len as u32,
        })
    }

    pub(crate) fn push(pool: &mut String, text: &str) -> Str {
        Str::push_with(pool, |pool| pool.push_str(text))
    }

    pub(crate) fn push_with(pool: &mut String, write: impl FnOnce(&mut String)) -> Str {
        if pool.capacity() == 0 {
            pool.reserve(FIRST_POOL);
        }
        let start = pool.len();
        write(pool);
        let end = pool.len();
        if end > MAX_LEN || start > u32::MAX as usize {
            pool.truncate(start);
            return Str::EMPTY;
        }
        Str {
            start: start as u32,
            len: (end - start) as u32 | POOL_BIT,
        }
    }

    #[inline]
    pub(crate) fn resolve<'a>(self, source: &'a [u8], pool: &'a str) -> &'a str {
        if self.is_none() {
            return "";
        }
        if self.is_pool() {
            return pool.get(self.range()).unwrap_or_default();
        }
        source.get(self.range()).map_or("", source_str)
    }

    pub(crate) fn resolve_option<'a>(self, source: &'a [u8], pool: &'a str) -> Option<&'a str> {
        self.is_some().then(|| self.resolve(source, pool))
    }
}

#[inline(never)]
fn is_utf8(bytes: &[u8]) -> bool {
    simdutf8::basic::from_utf8(bytes).is_ok()
}

pub(crate) fn validated_string(bytes: Vec<u8>) -> Result<String, Vec<u8>> {
    if is_utf8(&bytes) {
        Ok(owned_str(bytes))
    } else {
        Err(bytes)
    }
}

#[cfg(not(mail_parser_checked_str))]
#[allow(unsafe_code)]
fn source_str(bytes: &[u8]) -> &str {
    debug_assert!(simdutf8::basic::from_utf8(bytes).is_ok());
    // SAFETY: the only caller, `Str::resolve`, passes `source[range]` for a
    // span that is neither `NONE` nor in the pool. Only `Str::borrow` builds
    // such a span (the fields of `Str` are private to this module), after
    // checking that `source[range]` is ASCII or valid UTF-8 (simdutf8). The
    // crate resolves every span against the same immutable buffer it was
    // borrowed from: the raw input for message 0, the decoded
    // `SourceBuffer` of a nested message (`MessageData::message_source`,
    // with the message id of the part that owns the span), or the buffer
    // held by the `FieldCtx`, `Resolver` or `ParsedValue` that borrowed it.
    // `MessageData::clear` drops every span before the store is reused, and
    // a nested source is popped only when its message has no part, so no
    // span borrowed from it survives.
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

#[cfg(mail_parser_checked_str)]
fn source_str(bytes: &[u8]) -> &str {
    simdutf8::basic::from_utf8(bytes).unwrap_or_default()
}

#[cfg(not(mail_parser_checked_str))]
#[allow(unsafe_code)]
fn owned_str(bytes: Vec<u8>) -> String {
    // SAFETY: the only caller, `validated_string`, passes `bytes` after
    // `is_utf8` (simdutf8) accepted the whole vector, which is moved here
    // unchanged.
    unsafe { String::from_utf8_unchecked(bytes) }
}

#[cfg(mail_parser_checked_str)]
fn owned_str(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::Str;

    #[test]
    fn borrow_validates_utf8() {
        let source = "héllo wörld".as_bytes();
        let span = Str::borrow(source, 0..source.len()).expect("valid");
        assert_eq!(span.resolve(source, ""), "héllo wörld");
        assert!(Str::borrow(source, 0..2).is_none());
        assert!(Str::borrow(source, 0..100).is_none());
        assert!(Str::borrow(b"\xff\xfe", 0..2).is_none());
    }

    #[test]
    fn pool_strings() {
        let mut pool = String::new();
        let first = Str::push(&mut pool, "abc");
        let second = Str::push_with(&mut pool, |pool| pool.push_str("défg"));
        assert_eq!(first.resolve(b"", &pool), "abc");
        assert_eq!(second.resolve(b"", &pool), "défg");
        assert_eq!(Str::EMPTY.resolve(b"xyz", &pool), "");
        assert_eq!(Str::NONE.resolve_option(b"xyz", &pool), None);
        assert!(Str::EMPTY.is_empty() && Str::EMPTY.is_some());
    }
}
