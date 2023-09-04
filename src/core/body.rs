/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use crate::{AttachmentIterator, BodyPartIterator, Message, MessagePart, MessagePartId, PartType};

impl<'x> PartType<'x> {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            PartType::Text(v) | PartType::Html(v) => v.len(),
            PartType::Binary(v) | PartType::InlineBinary(v) => v.len(),
            PartType::Message(v) => v.raw_message.len(),
            PartType::Multipart(_) => 0,
        }
    }
}

impl<'x> BodyPartIterator<'x> {
    pub(crate) fn new(message: &'x Message<'x>, list: &'x [MessagePartId]) -> BodyPartIterator<'x> {
        BodyPartIterator {
            message,
            list,
            pos: -1,
        }
    }
}

impl<'x> Iterator for BodyPartIterator<'x> {
    type Item = &'x MessagePart<'x>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.message.parts.get(*self.list.get(self.pos as usize)?)
    }
}

impl<'x> AttachmentIterator<'x> {
    pub(crate) fn new(message: &'x Message<'x>) -> AttachmentIterator<'x> {
        AttachmentIterator { message, pos: -1 }
    }
}

impl<'x> Iterator for AttachmentIterator<'x> {
    type Item = &'x MessagePart<'x>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;
        self.message.attachment(self.pos as usize)
    }
}
