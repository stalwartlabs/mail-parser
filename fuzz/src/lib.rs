#[allow(dead_code, unreachable_pub)]
#[path = "../../src/scan/mod.rs"]
mod scan;

pub mod charsets;
pub mod fields;
pub mod kernels;
pub mod mbox;
pub mod walk;

use std::{
    fmt::{Debug, Display},
    io::{self, Write},
};

pub fn valid(text: &str) {
    assert!(
        std::str::from_utf8(text.as_bytes()).is_ok(),
        "invalid UTF-8 returned as &str"
    );
}

pub fn debug(value: &impl Debug) {
    let _ = write!(io::sink(), "{value:?}");
}

pub fn display(value: &impl Display) {
    let _ = write!(io::sink(), "{value}");
}

pub fn within(inner: &[u8], outer: &[u8]) -> bool {
    let (range, outer) = (inner.as_ptr_range(), outer.as_ptr_range());
    inner.is_empty() || range.start >= outer.start && range.end <= outer.end
}

pub fn char_prefix(text: &str, chars: usize) -> &str {
    text.char_indices()
        .nth(chars)
        .and_then(|(end, _)| text.get(..end))
        .unwrap_or(text)
}
