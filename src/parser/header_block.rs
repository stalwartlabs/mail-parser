/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{builder::Builder, delimiter::Delimiter};
use crate::{
    Encoding, HeaderForm,
    fields::{self, FieldCtx},
    header_name::{self, HeaderId},
    scan::swar::{self, WORD},
    store::{HeaderEntry, NONE, OTHER_NAME, Str, Value},
};
use memchr::memchr2;
use std::ops::Range;

const KEY_LEN: usize = 64;
const FIRST_NON_BLANK: u8 = 0x21;
const CONTENT_TYPE: u16 = HeaderId::ContentType as u16;
const CONTENT_DISPOSITION: u16 = HeaderId::ContentDisposition as u16;
const CONTENT_TRANSFER_ENCODING: u16 = HeaderId::ContentTransferEncoding as u16;

#[derive(Debug, Clone, Copy)]
pub(super) struct Mime {
    pub(super) content_type: u32,
    pub(super) disposition: u32,
    pub(super) encoding: Encoding,
}

impl Default for Mime {
    fn default() -> Self {
        Mime {
            content_type: NONE,
            disposition: NONE,
            encoding: Encoding::None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Terminator {
    Blank,
    Delimiter(Delimiter),
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Block {
    pub(super) body: usize,
    terminator: Terminator,
    pub(super) mime: Mime,
}

impl Block {
    pub(super) fn delimiter(&self) -> Option<Delimiter> {
        match self.terminator {
            Terminator::Delimiter(delimiter) => Some(delimiter),
            Terminator::Blank | Terminator::Eof => None,
        }
    }

    pub(super) fn reaches_eof(&self) -> bool {
        matches!(self.terminator, Terminator::Eof)
    }

    pub(super) fn has_blank_line(&self) -> bool {
        matches!(self.terminator, Terminator::Blank)
    }
}

fn is_line_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\x0c')
}

struct FieldName {
    name: Range<usize>,
    colon: usize,
    id: Option<u16>,
}

fn field_name(field: &[u8]) -> Result<FieldName, usize> {
    match simple_name(field) {
        Some(name) => Ok(name),
        None => {
            let (name, colon) = split_name(field)?;
            Ok(FieldName {
                id: header_name::lookup(field.get(name.clone()).unwrap_or_default()),
                name,
                colon,
            })
        }
    }
}

fn simple_name(field: &[u8]) -> Option<FieldName> {
    let mut key = [0u8; KEY_LEN];
    for (index, (bytes, out)) in field.chunks(WORD).zip(key.chunks_mut(WORD)).enumerate() {
        let word = swar::load(bytes);
        out.copy_from_slice(&swar::ascii_lowercase(word).to_le_bytes());
        let stops = swar::equal_bytes(word, b':') | swar::bytes_below(word, FIRST_NON_BLANK);
        if stops == 0 {
            continue;
        }
        let colon = index * WORD + swar::first_byte(stops);
        return (colon > 0 && field.get(colon) == Some(&b':')).then(|| FieldName {
            name: 0..colon,
            colon,
            id: key.get(..colon).and_then(header_name::lookup_lowercase),
        });
    }
    None
}

fn split_name(field: &[u8]) -> Result<(Range<usize>, usize), usize> {
    let lead = field.iter().take_while(|&&byte| byte == b':').count();
    let rest = field.get(lead..).unwrap_or_default();
    match memchr2(b':', b'\n', rest) {
        Some(colon) if rest.get(colon) == Some(&b':') => {
            let name = field.get(..lead + colon).unwrap_or_default();
            Ok((0..header_name::trim_blank_end(name).len(), lead + colon))
        }
        Some(newline) => Err(lead + newline + 1),
        None => Err(field.len()),
    }
}

enum Fold {
    Field(usize, FieldName),
    End(usize, Terminator),
    Continue,
}

#[inline(never)]
fn folded_field(src: &[u8], mut line: usize, field_end: usize) -> Fold {
    while line < field_end {
        let rest = src.get(line..field_end).unwrap_or_default();
        let skip = rest.iter().take_while(|&&byte| is_line_space(byte)).count();
        match rest.get(skip) {
            None if field_end == src.len() => return Fold::End(src.len(), Terminator::Eof),
            None => return Fold::Continue,
            Some(b'\n') => return Fold::End(line + skip + 1, Terminator::Blank),
            Some(_) => {}
        }
        let field_start = line + skip;
        match field_name(src.get(field_start..field_end).unwrap_or_default()) {
            Ok(name) => return Fold::Field(field_start, name),
            Err(next_line) => line = field_start + next_line,
        }
    }
    Fold::Continue
}

impl Builder<'_, '_> {
    pub(super) fn header_block(&mut self, src: &[u8], offset: usize) -> Block {
        let mut mime = Mime::default();
        let mut line = offset;
        loop {
            let rest = src.get(line..).unwrap_or_default();
            if rest.is_empty() {
                return Block {
                    body: src.len(),
                    terminator: Terminator::Eof,
                    mime,
                };
            }
            if self.has_open_boundaries()
                && rest.starts_with(b"--")
                && let Some(delimiter) = self.delimiter_at(src, line)
            {
                return Block {
                    body: delimiter.line,
                    terminator: Terminator::Delimiter(delimiter),
                    mime,
                };
            }
            let skip = rest.iter().take_while(|&&byte| is_line_space(byte)).count();
            match rest.get(skip) {
                None => {
                    return Block {
                        body: src.len(),
                        terminator: Terminator::Eof,
                        mime,
                    };
                }
                Some(b'\n') => {
                    return Block {
                        body: line + skip + 1,
                        terminator: Terminator::Blank,
                        mime,
                    };
                }
                Some(_) => {}
            }
            let field_start = line + skip;
            let field_end = self
                .kernel
                .field_end(src, field_start)
                .map_or(src.len(), |newline| newline + 1);
            let field = src.get(field_start..field_end).unwrap_or_default();
            let (field_start, name) = match field_name(field) {
                Ok(name) => (field_start, name),
                Err(next_line) => match folded_field(src, field_start + next_line, field_end) {
                    Fold::Field(start, name) => (start, name),
                    Fold::End(body, terminator) => {
                        return Block {
                            body,
                            terminator,
                            mime,
                        };
                    }
                    Fold::Continue => {
                        line = field_end;
                        continue;
                    }
                },
            };
            self.push_header(
                src,
                &name,
                field_start,
                field_start + name.colon + 1..field_end,
                &mut mime,
            );
            line = field_end;
        }
    }

    fn push_header(
        &mut self,
        src: &[u8],
        name: &FieldName,
        field_start: usize,
        value: Range<usize>,
        mime: &mut Mime,
    ) {
        let id = name.id;
        let name_bytes = src
            .get(field_start + name.name.start..field_start + name.name.end)
            .unwrap_or_default();
        let form = match id {
            Some(id) => self.conf.known_form(id),
            None => self.conf.other_form(name_bytes),
        };
        let other_name = match id {
            Some(_) => NONE,
            None => {
                let text = match Str::borrow(
                    src,
                    field_start + name.name.start..field_start + name.name.end,
                ) {
                    Some(text) => text,
                    None => Str::push_with(&mut self.data.strings, |pool| {
                        fields::push_utf8_lossy(pool, name_bytes)
                    }),
                };
                self.data.push_text_item(text)
            }
        };
        let parsed = match form {
            HeaderForm::Ignore => Value::Empty,
            form => form.parse_field(
                &mut FieldCtx::with_kernel(src, self.data, self.kernel),
                value.clone(),
            ),
        };
        self.data.headers.push(HeaderEntry {
            offset_field: field_start as u32,
            offset_start: value.start as u32,
            offset_end: value.end as u32,
            value: parsed,
            name: id.unwrap_or(OTHER_NAME),
            other_name,
        });
        let content_type = match parsed {
            Value::ContentType(index) => index,
            _ => NONE,
        };
        match id {
            Some(CONTENT_TYPE) => mime.content_type = content_type,
            Some(CONTENT_DISPOSITION) => mime.disposition = content_type,
            Some(CONTENT_TRANSFER_ENCODING) => {
                mime.encoding =
                    Encoding::parse(src.get(value).unwrap_or_default()).unwrap_or_default();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{field_name, split_name};
    use crate::{Encoding, header_name, scan::tests::Rng};
    use std::ops::Range;

    type Slow = Result<(Range<usize>, usize, Option<u16>), usize>;

    fn slow(field: &[u8]) -> Slow {
        let (name, colon) = split_name(field)?;
        let bytes = field.get(name.clone()).unwrap_or_default();
        Ok((name, colon, header_name::lookup(bytes)))
    }

    fn fast(field: &[u8]) -> Slow {
        field_name(field).map(|name| (name.name, name.colon, name.id))
    }

    #[test]
    fn field_names_match_split_name() {
        let long = "X-".repeat(40);
        let mut names: Vec<Vec<u8>> = [
            &b"Subject"[..],
            b"subject",
            b"SUBJECT",
            b"Content-Transfer-Encoding",
            b"content-type",
            b"X-Mailer",
            b"X-MS-Exchange-Organization-AuthAs",
            b"x",
            b"Sub ject",
            b"Subject ",
            b"Subject\t",
            b":Subject",
            b"::",
            b"caf\xc3\xa9",
            b"X-\xff",
            b"X-\x01-Y",
            b"X-\x7f",
            b"Received",
            b"A@[]{}~",
            b"",
        ]
        .iter()
        .map(|name| name.to_vec())
        .collect();
        names.extend((55..=70).map(|len| long.as_bytes().get(..len).unwrap_or_default().to_vec()));
        names.extend((55..=70).map(|len| {
            let mut name = b"Subject".to_vec();
            name.resize(len, b'x');
            name
        }));
        for name in &names {
            for tail in [
                &b": value\r\n"[..],
                b":value\n",
                b" : v\n",
                b"\t: v\n",
                b":",
                b"",
                b"\n next: x\n",
                b"\r\n",
            ] {
                let mut field = name.clone();
                field.extend_from_slice(tail);
                for end in 0..=field.len() {
                    let prefix = field.get(..end).unwrap_or_default();
                    assert_eq!(
                        fast(prefix),
                        slow(prefix),
                        "{:?}",
                        String::from_utf8_lossy(prefix)
                    );
                }
            }
        }
        let alphabet = b"aZ-:: \t\r\n\x00\x80\xc3\xa9!~";
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        for _ in 0..50_000 {
            let len = rng.below(80);
            let field: Vec<u8> = (0..len)
                .map(|_| {
                    alphabet
                        .get(rng.below(alphabet.len()))
                        .copied()
                        .unwrap_or(b'a')
                })
                .collect();
            assert_eq!(fast(&field), slow(&field), "{field:?}");
        }
    }

    #[test]
    fn names() {
        assert_eq!(split_name(b"Subject: x\n"), Ok((0..7, 7)));
        assert_eq!(split_name(b"subject   : x\n"), Ok((0..7, 10)));
        assert_eq!(split_name(b"Subject\t \t: x\n"), Ok((0..7, 10)));
        assert_eq!(split_name(b"From\r: x\n"), Ok((0..5, 5)));
        assert_eq!(split_name(b":Weird: v"), Ok((0..6, 6)));
        assert_eq!(split_name(b"::  From: v"), Ok((0..8, 8)));
        assert_eq!(split_name(b"mal formed: v"), Ok((0..10, 10)));
        assert_eq!(split_name(b": only colon\n next: x\n"), Err(13));
        assert_eq!(split_name(b"no colon here"), Err(13));
        assert_eq!(split_name(b"no colon\n here: x"), Err(9));
        assert_eq!(split_name(b""), Err(0));
    }

    #[test]
    fn transfer_encodings() {
        assert_eq!(
            Encoding::parse(b" base64\r\n").unwrap_or_default(),
            Encoding::Base64
        );
        assert_eq!(
            Encoding::parse(b" BASE64 (comment)\n").unwrap_or_default(),
            Encoding::Base64
        );
        assert_eq!(
            Encoding::parse(b"\r\n Quoted-Printable\r\n").unwrap_or_default(),
            Encoding::QuotedPrintable
        );
        assert_eq!(
            Encoding::parse(b" 8bit\n").unwrap_or_default(),
            Encoding::None
        );
        assert_eq!(
            Encoding::parse(b" base64;\n").unwrap_or_default(),
            Encoding::None
        );
    }
}
