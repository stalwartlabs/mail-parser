/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{base64_feed, charsets::Charset};
use crate::Encoding;
use encodify::qp;
use memchr::memrchr;
use std::borrow::Cow;

const MAX_BYTES_PER_CHAR: usize = 4;
const RAW_BYTES_PER_TEXT_BYTE: usize = 2;
const INITIAL_SLACK: usize = 256;
const GROWTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Limit {
    Chars(usize),
    Bytes(usize),
}

pub(crate) struct TextPrefix<'x> {
    pub(crate) text: Cow<'x, str>,
    pub(crate) complete: bool,
}

impl<'x> TextPrefix<'x> {
    pub(crate) fn decode(
        raw: &'x [u8],
        encoding: Encoding,
        charset: Charset,
        limit: Limit,
    ) -> TextPrefix<'x> {
        let mut window = match limit {
            Limit::Chars(chars) => chars.saturating_mul(MAX_BYTES_PER_CHAR),
            Limit::Bytes(bytes) => bytes.saturating_mul(RAW_BYTES_PER_TEXT_BYTE),
        }
        .saturating_add(INITIAL_SLACK);
        loop {
            let whole = window >= raw.len();
            let text = if whole {
                charset.decode_cow(encoding.decode(raw))
            } else {
                let prefix = raw.get(..window).unwrap_or(raw);
                charset.decode_prefix_cow(encoding.decode_prefix(prefix))
            };
            match limit {
                Limit::Chars(chars) => {
                    if let Some(end) = char_start(&text, chars) {
                        return TextPrefix {
                            text: truncate(text, end),
                            complete: false,
                        };
                    }
                }
                Limit::Bytes(bytes) if text.len() > bytes => {
                    return TextPrefix {
                        text,
                        complete: whole,
                    };
                }
                Limit::Bytes(_) => (),
            }
            if whole {
                return TextPrefix {
                    text,
                    complete: true,
                };
            }
            window = window.saturating_mul(GROWTH);
        }
    }
}

fn char_start(text: &str, index: usize) -> Option<usize> {
    if index >= text.len() {
        None
    } else if text.as_bytes().get(..index).is_some_and(<[u8]>::is_ascii) {
        Some(index)
    } else {
        text.char_indices().nth(index).map(|(start, _)| start)
    }
}

impl Encoding {
    fn decode_prefix(self, raw: &[u8]) -> Cow<'_, [u8]> {
        match self {
            Encoding::None => Cow::Borrowed(raw),
            Encoding::QuotedPrintable => {
                let lines = raw
                    .get(..memrchr(b'\n', raw).map_or(0, |end| end + 1))
                    .unwrap_or_default();
                qp::BODY.decode(lines).unwrap_or(Cow::Borrowed(lines))
            }
            Encoding::Base64 => {
                let mut out = Vec::with_capacity(raw.len() / 4 * 3 + 3);
                base64_feed(raw, &mut out);
                Cow::Owned(out)
            }
        }
    }
}

impl Charset {
    fn decode_prefix_cow(self, bytes: Cow<'_, [u8]>) -> Cow<'_, str> {
        match bytes {
            Cow::Borrowed(bytes) => self.decode_prefix(bytes),
            Cow::Owned(bytes) => Cow::Owned(self.decode_prefix(&bytes).into_owned()),
        }
    }
}

fn truncate(text: Cow<'_, str>, end: usize) -> Cow<'_, str> {
    match text {
        Cow::Borrowed(text) => Cow::Borrowed(text.get(..end).unwrap_or(text)),
        Cow::Owned(mut text) => {
            text.truncate(end);
            Cow::Owned(text)
        }
    }
}
