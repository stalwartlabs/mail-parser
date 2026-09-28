/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::store::{KindTag, role};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MimeType {
    MultipartMixed,
    MultipartAlternative,
    MultipartRelated,
    MultipartDigest,
    TextPlain,
    TextHtml,
    TextOther,
    Inline,
    #[default]
    Message,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Disposition {
    #[default]
    None,
    Inline,
    Attachment,
    Other,
}

impl Disposition {
    pub(crate) fn new(disposition_type: Option<&str>) -> Self {
        match disposition_type {
            None => Disposition::None,
            Some(value) if value.eq_ignore_ascii_case("inline") => Disposition::Inline,
            Some(value) if value.eq_ignore_ascii_case("attachment") => Disposition::Attachment,
            Some(_) => Disposition::Other,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MimeClass {
    pub(crate) is_multipart: bool,
    pub(crate) is_inline: bool,
    pub(crate) is_text: bool,
    pub(crate) mime_type: MimeType,
}

impl MimeClass {
    pub(crate) fn new(content_type: Option<(&str, Option<&str>)>, parent: MimeType) -> Self {
        let (is_multipart, is_inline, is_text, mime_type) = match content_type {
            Some((ctype, subtype)) => match ctype {
                "multipart" => (
                    true,
                    false,
                    false,
                    match subtype {
                        Some("mixed") => MimeType::MultipartMixed,
                        Some("alternative") => MimeType::MultipartAlternative,
                        Some("related") => MimeType::MultipartRelated,
                        Some("digest") => MimeType::MultipartDigest,
                        _ => MimeType::Other,
                    },
                ),
                "text" => match subtype {
                    Some("plain") => (false, true, true, MimeType::TextPlain),
                    Some("html") => (false, true, true, MimeType::TextHtml),
                    _ => (false, false, true, MimeType::TextOther),
                },
                "image" | "audio" | "video" => (false, true, false, MimeType::Inline),
                "message" if matches!(subtype, Some("rfc822" | "global")) => {
                    (false, false, false, MimeType::Message)
                }
                _ => (false, false, false, MimeType::Other),
            },
            None if parent == MimeType::MultipartDigest => (false, false, false, MimeType::Message),
            None => (false, true, true, MimeType::TextPlain),
        };
        MimeClass {
            is_multipart,
            is_inline,
            is_text,
            mime_type,
        }
    }

    pub(crate) fn as_broken_text(self) -> Self {
        MimeClass {
            is_multipart: false,
            is_inline: false,
            is_text: true,
            mime_type: if self.mime_type == MimeType::TextPlain {
                MimeType::TextPlain
            } else {
                MimeType::TextOther
            },
        }
    }

    pub(crate) fn is_text_body(self) -> bool {
        matches!(self.mime_type, MimeType::TextPlain | MimeType::TextHtml)
    }

    pub(crate) fn plain_kind(self) -> KindTag {
        match self.mime_type {
            _ if self.is_multipart => KindTag::Multipart,
            MimeType::TextHtml => KindTag::Html,
            _ if self.is_text => KindTag::Text,
            _ => KindTag::Binary,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Container {
    pub(crate) mime_type: MimeType,
    pub(crate) in_alternative: bool,
    pub(crate) parts: u32,
    pub(crate) html_parts: u32,
    pub(crate) text_parts: u32,
    pub(crate) need_html_body: bool,
    pub(crate) need_text_body: bool,
}

pub(crate) struct Leaf {
    pub(crate) class: MimeClass,
    pub(crate) disposition: Disposition,
    pub(crate) has_part_name: bool,
}

pub(crate) struct Classified {
    pub(crate) kind: KindTag,
    pub(crate) roles: u16,
}

impl Container {
    pub(crate) fn message() -> Self {
        Container {
            mime_type: MimeType::Message,
            in_alternative: false,
            parts: 0,
            html_parts: 0,
            text_parts: 0,
            need_html_body: true,
            need_text_body: true,
        }
    }

    pub(crate) fn multipart(&self, mime_type: MimeType, html_len: u32, text_len: u32) -> Self {
        Container {
            in_alternative: self.in_alternative || mime_type == MimeType::MultipartAlternative,
            mime_type,
            parts: 0,
            html_parts: html_len,
            text_parts: text_len,
            need_html_body: self.need_html_body,
            need_text_body: self.need_text_body,
        }
    }

    fn is_first(&self) -> bool {
        self.parts == 1
    }

    fn renders_inline(&self, disposition: Disposition) -> bool {
        match disposition {
            Disposition::Inline => true,
            Disposition::None => self.mime_type == MimeType::MultipartRelated && !self.is_first(),
            Disposition::Attachment | Disposition::Other => false,
        }
    }

    fn inline_role(&self, disposition: Disposition) -> u16 {
        if self.renders_inline(disposition) {
            role::INLINE
        } else {
            0
        }
    }

    pub(crate) fn message_roles(&self, disposition: Disposition) -> u16 {
        role::ATTACHMENT | self.inline_role(disposition)
    }

    pub(crate) fn classify(&mut self, leaf: &Leaf) -> Classified {
        let lists = self.rfc8621_lists(leaf);
        let inline = self.inline_role(leaf.disposition);
        let kind = match leaf.class.mime_type {
            MimeType::TextHtml if leaf.class.is_text => KindTag::Html,
            _ if leaf.class.is_text => KindTag::Text,
            _ if inline != 0 => KindTag::InlineBinary,
            _ => KindTag::Binary,
        };
        Classified {
            kind,
            roles: lists | inline,
        }
    }

    fn rfc8621_lists(&mut self, leaf: &Leaf) -> u16 {
        let mime_type = leaf.class.mime_type;
        let is_media = mime_type == MimeType::Inline;
        let is_inline = leaf.class.is_inline
            && leaf.disposition != Disposition::Attachment
            && (self.is_first()
                || self.mime_type != MimeType::MultipartRelated
                    && (is_media || !leaf.has_part_name));
        if !is_inline {
            return role::ATTACHMENT;
        }
        if self.mime_type == MimeType::MultipartAlternative {
            return match mime_type {
                MimeType::TextPlain => list_roles(self.need_text_body, false, false),
                MimeType::TextHtml => list_roles(false, self.need_html_body, false),
                _ => role::ATTACHMENT,
            };
        }
        if self.in_alternative {
            match mime_type {
                MimeType::TextPlain => self.need_html_body = false,
                MimeType::TextHtml => self.need_text_body = false,
                _ => (),
            }
        }
        list_roles(
            self.need_text_body,
            self.need_html_body,
            is_media && !(self.need_text_body && self.need_html_body),
        )
    }

    pub(crate) fn needs_alternative_fixup(&self) -> bool {
        self.mime_type == MimeType::MultipartAlternative
            && self.need_html_body
            && self.need_text_body
    }
}

fn list_roles(text_body: bool, html_body: bool, attachment: bool) -> u16 {
    let mut roles = 0;
    if text_body {
        roles |= role::IN_TEXT_BODY;
    }
    if html_body {
        roles |= role::IN_HTML_BODY;
    }
    if attachment {
        roles |= role::ATTACHMENT;
    }
    roles
}
