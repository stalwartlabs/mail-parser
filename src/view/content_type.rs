/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::Resolver;
use crate::store::{ContentTypeEntry, MessageData, ParamEntry};
use std::fmt;

struct Attributes<'m>(ContentType<'m>);

/// A Content-Type or Content-Disposition value. Type, subtype and
/// parameter names are lowercase; RFC 2231 and RFC 2047 encoded parameter
/// values are decoded. Values compare by content.
///
/// ```
/// use mail_parser::MessageParser;
///
/// let raw = b"Content-Type: Text/Plain; Charset=\"utf-8\"; format=flowed\r\n\r\nbody";
/// let message = MessageParser::new().parse(raw).expect("message");
/// let content_type = message.content_type().expect("content type");
/// assert_eq!((content_type.ctype(), content_type.subtype()), ("text", Some("plain")));
/// assert_eq!(content_type.attribute("charset"), Some("utf-8"));
/// assert!(message.is_content_type("text", "plain"));
/// ```
#[derive(Clone, Copy)]
pub struct ContentType<'m> {
    resolver: Resolver<'m>,
    entry: &'m ContentTypeEntry,
    params: &'m [ParamEntry],
}

impl<'m> ContentType<'m> {
    #[inline]
    pub(crate) fn new(
        resolver: Resolver<'m>,
        data: &'m MessageData,
        entry: &'m ContentTypeEntry,
    ) -> Self {
        ContentType {
            resolver,
            entry,
            params: data.params.get(entry.params.range()).unwrap_or_default(),
        }
    }

    /// The type (`text`, `multipart`, `attachment`, ...).
    #[inline]
    pub fn ctype(&self) -> &'m str {
        self.resolver.str(self.entry.ctype)
    }

    /// The subtype (`plain`, `mixed`, ...); `None` for a disposition or a
    /// type without `/`.
    #[inline]
    pub fn subtype(&self) -> Option<&'m str> {
        self.resolver.opt(self.entry.subtype)
    }

    /// The first parameter with this (lowercase) name.
    pub fn attribute(&self, name: &str) -> Option<&'m str> {
        self.attributes()
            .find(|(param, _)| *param == name)
            .map(|(_, value)| value)
    }

    /// Every parameter in document order.
    #[inline]
    pub fn attributes(
        &self,
    ) -> impl ExactSizeIterator<Item = (&'m str, &'m str)> + DoubleEndedIterator + use<'m> {
        let resolver = self.resolver;
        self.params
            .iter()
            .map(move |param| (resolver.str(param.name), resolver.str(param.value)))
    }

    /// Whether a parameter with this (lowercase) name is present.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attribute(name).is_some()
    }

    /// Whether this is `Content-Disposition: attachment`.
    pub fn is_attachment(&self) -> bool {
        self.ctype().eq_ignore_ascii_case("attachment")
    }

    /// Whether this is `Content-Disposition: inline`.
    pub fn is_inline(&self) -> bool {
        self.ctype().eq_ignore_ascii_case("inline")
    }
}

impl fmt::Debug for Attributes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.0.attributes()).finish()
    }
}

impl fmt::Debug for ContentType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContentType")
            .field("type", &self.ctype())
            .field("subtype", &self.subtype())
            .field("attributes", &Attributes(*self))
            .finish()
    }
}

impl PartialEq for ContentType<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.ctype() == other.ctype()
            && self.subtype() == other.subtype()
            && self.params.len() == other.params.len()
            && self.attributes().eq(other.attributes())
    }
}

impl Eq for ContentType<'_> {}
