/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::FieldCtx;
use crate::{
    DateTime, Greeting, Protocol, TlsVersion,
    store::{HostEntry, ReceivedEntry, Str, Value},
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Range,
};

pub(crate) fn parse_received(ctx: &mut FieldCtx<'_>, value: Range<usize>) -> Value {
    let entry = Grammar::new(ctx, value).run();
    if entry == ReceivedEntry::EMPTY {
        Value::Empty
    } else {
        ctx.received(entry)
    }
}

const LOWER_ALPHA: u16 = 1;
const UPPER_ALPHA: u16 = 1 << 1;
const LOWER_HEX: u16 = 1 << 2;
const UPPER_HEX: u16 = 1 << 3;
const DIGIT: u16 = 1 << 4;
const DOT: u16 = 1 << 5;
const AT: u16 = 1 << 6;
const PLUS: u16 = 1 << 7;
const MINUS: u16 = 1 << 8;
const UNDERSCORE: u16 = 1 << 9;
const OTHER: u16 = 1 << 10;
const UTF: u16 = 1 << 11;
const COLON: u16 = 1 << 12;
const SPACE: u16 = 1 << 13;
const SEPARATOR: u16 = 1 << 14;

const ALPHA: u16 = LOWER_ALPHA | UPPER_ALPHA;
const HEX: u16 = LOWER_HEX | UPPER_HEX;
const LETTER: u16 = ALPHA | HEX;
const LOWER: u16 = LOWER_ALPHA | LOWER_HEX;
const ALNUM: u16 = LETTER | DIGIT;
const HEX_DIGIT: u16 = HEX | DIGIT;
const WORD: u16 = ALNUM | DOT | AT | PLUS | MINUS | UNDERSCORE | OTHER | UTF;

const IN_COMMENT: u8 = 1;
const IN_BRACKET: u8 = 1 << 1;

const MAX_VALUE: usize = u32::MAX as usize;
const MAX_COLONS: u8 = 7;
const UNSET: i64 = i64::MAX;
const MAX_KEYWORD: usize = 11;
const MIN_CIPHER: usize = 7;
const DAY_NAMES: [&[u8; 3]; 7] = [b"mon", b"tue", b"wed", b"thu", b"fri", b"sat", b"sun"];
const CIPHER_PREFIXES: [&[u8; 3]; 8] = [
    b"rsa", b"ecd", b"dhe", b"psk", b"srp", b"aes", b"des", b"tls",
];

static CLASS: [u16; 256] = {
    let mut table = [0; 256];
    let mut byte = 0;
    while byte < table.len() {
        table[byte] = byte_class(byte as u8);
        byte += 1;
    }
    table
};

const fn byte_class(byte: u8) -> u16 {
    match byte {
        b'0'..=b'9' => DIGIT,
        b'a'..=b'f' => LOWER_HEX,
        b'g'..=b'z' => LOWER_ALPHA,
        b'A'..=b'F' => UPPER_HEX,
        b'G'..=b'Z' => UPPER_ALPHA,
        b'.' => DOT,
        b'@' => AT,
        b'+' => PLUS,
        b'-' => MINUS,
        b'_' => UNDERSCORE,
        b':' => COLON,
        b' ' | b'\t' | b'\r' | b'\n' => SPACE,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'=' | b';' | b'/' | b'"' | b',' => SEPARATOR,
        0x7f..=0xff => UTF,
        _ => OTHER,
    }
}

fn class(byte: u8) -> u16 {
    CLASS[usize::from(byte)]
}

const KEYWORDS: [(&[u8], Kind); 78] = [
    (b"from", Kind::From),
    (b"by", Kind::By),
    (b"for", Kind::For),
    (b"id", Kind::Id),
    (b"via", Kind::Via),
    (b"with", Kind::With),
    (b"ident", Kind::Ident),
    (b"helo", Kind::Greeting(Greeting::Helo)),
    (b"ehlo", Kind::Greeting(Greeting::Ehlo)),
    (b"lhlo", Kind::Greeting(Greeting::Lhlo)),
    (b"jan", Kind::Month(1)),
    (b"feb", Kind::Month(2)),
    (b"mar", Kind::Month(3)),
    (b"apr", Kind::Month(4)),
    (b"may", Kind::Month(5)),
    (b"jun", Kind::Month(6)),
    (b"jul", Kind::Month(7)),
    (b"aug", Kind::Month(8)),
    (b"sep", Kind::Month(9)),
    (b"oct", Kind::Month(10)),
    (b"nov", Kind::Month(11)),
    (b"dec", Kind::Month(12)),
    (b"smtp", Kind::Protocol(Protocol::SMTP)),
    (b"bsmtp", Kind::Protocol(Protocol::SMTP)),
    (b"smtpd", Kind::Protocol(Protocol::SMTP)),
    (b"smtpsvc", Kind::Protocol(Protocol::SMTP)),
    (b"localbsmtp", Kind::Protocol(Protocol::SMTP)),
    (b"esmtp", Kind::Protocol(Protocol::ESMTP)),
    (b"localesmtp", Kind::Protocol(Protocol::ESMTP)),
    (b"esmtpa", Kind::Protocol(Protocol::ESMTPA)),
    (b"asmtp", Kind::Protocol(Protocol::ESMTPA)),
    (b"esmtps", Kind::Protocol(Protocol::ESMTPS)),
    (b"esmtptls", Kind::Protocol(Protocol::ESMTPS)),
    (b"localesmtps", Kind::Protocol(Protocol::ESMTPS)),
    (b"esmtpsa", Kind::Protocol(Protocol::ESMTPSA)),
    (b"lmtp", Kind::Protocol(Protocol::LMTP)),
    (b"lsmtp", Kind::Protocol(Protocol::LMTP)),
    (b"lmtpa", Kind::Protocol(Protocol::LMTPA)),
    (b"lmtps", Kind::Protocol(Protocol::LMTPS)),
    (b"lmtpsa", Kind::Protocol(Protocol::LMTPSA)),
    (b"mms", Kind::Protocol(Protocol::MMS)),
    (b"utf8smtp", Kind::Protocol(Protocol::UTF8SMTP)),
    (b"utf8smtpa", Kind::Protocol(Protocol::UTF8SMTPA)),
    (b"utf8smtps", Kind::Protocol(Protocol::UTF8SMTPS)),
    (b"utf8smtpsa", Kind::Protocol(Protocol::UTF8SMTPSA)),
    (b"utf8lmtp", Kind::Protocol(Protocol::UTF8LMTP)),
    (b"utf8lmtpa", Kind::Protocol(Protocol::UTF8LMTPA)),
    (b"utf8lmtps", Kind::Protocol(Protocol::UTF8LMTPS)),
    (b"utf8lmtpsa", Kind::Protocol(Protocol::UTF8LMTPSA)),
    (b"http", Kind::Protocol(Protocol::HTTP)),
    (b"httpu", Kind::Protocol(Protocol::HTTP)),
    (b"httprest", Kind::Protocol(Protocol::HTTP)),
    (b"https", Kind::Protocol(Protocol::HTTPS)),
    (b"imap", Kind::Protocol(Protocol::IMAP)),
    (b"pop3", Kind::Protocol(Protocol::POP3)),
    (b"local", Kind::Protocol(Protocol::Local)),
    (b"socket", Kind::Protocol(Protocol::Local)),
    (b"stdin", Kind::Protocol(Protocol::Local)),
    (b"ssl2", Kind::Tls(TlsVersion::SSLv2)),
    (b"sslv2", Kind::Tls(TlsVersion::SSLv2)),
    (b"ssl3", Kind::Tls(TlsVersion::SSLv3)),
    (b"sslv3", Kind::Tls(TlsVersion::SSLv3)),
    (b"tls1", Kind::Tls(TlsVersion::TLSv1_0)),
    (b"tlsv1", Kind::Tls(TlsVersion::TLSv1_0)),
    (b"tls10", Kind::Tls(TlsVersion::TLSv1_0)),
    (b"tlsv10", Kind::Tls(TlsVersion::TLSv1_0)),
    (b"tls11", Kind::Tls(TlsVersion::TLSv1_1)),
    (b"tlsv11", Kind::Tls(TlsVersion::TLSv1_1)),
    (b"tls12", Kind::Tls(TlsVersion::TLSv1_2)),
    (b"tlsv12", Kind::Tls(TlsVersion::TLSv1_2)),
    (b"tls13", Kind::Tls(TlsVersion::TLSv1_3)),
    (b"tlsv13", Kind::Tls(TlsVersion::TLSv1_3)),
    (b"dtls10", Kind::Tls(TlsVersion::DTLSv1_0)),
    (b"dtlsv10", Kind::Tls(TlsVersion::DTLSv1_0)),
    (b"dtls12", Kind::Tls(TlsVersion::DTLSv1_2)),
    (b"dtlsv12", Kind::Tls(TlsVersion::DTLSv1_2)),
    (b"dtls13", Kind::Tls(TlsVersion::DTLSv1_3)),
    (b"dtlsv13", Kind::Tls(TlsVersion::DTLSv1_3)),
];

static KEYWORD_STARTS: [u32; MAX_KEYWORD + 1] = {
    let mut starts = [0; MAX_KEYWORD + 1];
    let mut index = 0;
    while index < KEYWORDS.len() {
        let keyword = KEYWORDS[index].0;
        starts[keyword.len()] |= 1 << (keyword[0] - b'a');
        index += 1;
    }
    starts
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    BracketOpen,
    BracketClose,
    AngleOpen,
    AngleClose,
    ParenOpen,
    ParenClose,
    Semicolon,
    Colon,
    Equal,
    Slash,
    Quote,
    Comma,
    From,
    By,
    For,
    Id,
    Via,
    With,
    Ident,
    Greeting(Greeting),
    Protocol(Protocol),
    Tls(TlsVersion),
    Month(u8),
    Word,
}

impl Kind {
    fn is_separator(self) -> bool {
        matches!(
            self,
            Kind::BracketOpen
                | Kind::BracketClose
                | Kind::AngleOpen
                | Kind::AngleClose
                | Kind::ParenOpen
                | Kind::ParenClose
                | Kind::Semicolon
                | Kind::Colon
                | Kind::Equal
                | Kind::Slash
                | Kind::Quote
                | Kind::Comma
        )
    }

    fn is_inert(self) -> bool {
        self != Kind::Semicolon && self.is_separator()
    }

    fn of_word(text: &[u8], mask: u16, colons: u8) -> Kind {
        if colons != 0 || mask & LETTER == 0 || mask & (AT | PLUS | UTF) != 0 {
            Kind::Word
        } else if mask & !ALNUM == 0 {
            Kind::keyword(text)
        } else {
            Kind::decorated_keyword(text, mask)
        }
    }

    fn keyword(text: &[u8]) -> Kind {
        let starts = KEYWORD_STARTS.get(text.len()).copied().unwrap_or_default();
        let letter = text
            .first()
            .map_or(u8::MAX, |byte| (byte | 0x20).wrapping_sub(b'a'));
        if letter < 26 && starts & (1 << letter) != 0 {
            Kind::lookup(text)
        } else {
            Kind::Word
        }
    }

    fn lookup(text: &[u8]) -> Kind {
        hashify::map_ignore_case!(text, Kind,
            "from" => Kind::From,
            "by" => Kind::By,
            "for" => Kind::For,
            "id" => Kind::Id,
            "via" => Kind::Via,
            "with" => Kind::With,
            "ident" => Kind::Ident,
            "helo" => Kind::Greeting(Greeting::Helo),
            "ehlo" => Kind::Greeting(Greeting::Ehlo),
            "lhlo" => Kind::Greeting(Greeting::Lhlo),
            "jan" => Kind::Month(1),
            "feb" => Kind::Month(2),
            "mar" => Kind::Month(3),
            "apr" => Kind::Month(4),
            "may" => Kind::Month(5),
            "jun" => Kind::Month(6),
            "jul" => Kind::Month(7),
            "aug" => Kind::Month(8),
            "sep" => Kind::Month(9),
            "oct" => Kind::Month(10),
            "nov" => Kind::Month(11),
            "dec" => Kind::Month(12),
            "smtp" => Kind::Protocol(Protocol::SMTP),
            "bsmtp" => Kind::Protocol(Protocol::SMTP),
            "smtpd" => Kind::Protocol(Protocol::SMTP),
            "smtpsvc" => Kind::Protocol(Protocol::SMTP),
            "localbsmtp" => Kind::Protocol(Protocol::SMTP),
            "esmtp" => Kind::Protocol(Protocol::ESMTP),
            "localesmtp" => Kind::Protocol(Protocol::ESMTP),
            "esmtpa" => Kind::Protocol(Protocol::ESMTPA),
            "asmtp" => Kind::Protocol(Protocol::ESMTPA),
            "esmtps" => Kind::Protocol(Protocol::ESMTPS),
            "esmtptls" => Kind::Protocol(Protocol::ESMTPS),
            "localesmtps" => Kind::Protocol(Protocol::ESMTPS),
            "esmtpsa" => Kind::Protocol(Protocol::ESMTPSA),
            "lmtp" => Kind::Protocol(Protocol::LMTP),
            "lsmtp" => Kind::Protocol(Protocol::LMTP),
            "lmtpa" => Kind::Protocol(Protocol::LMTPA),
            "lmtps" => Kind::Protocol(Protocol::LMTPS),
            "lmtpsa" => Kind::Protocol(Protocol::LMTPSA),
            "mms" => Kind::Protocol(Protocol::MMS),
            "utf8smtp" => Kind::Protocol(Protocol::UTF8SMTP),
            "utf8smtpa" => Kind::Protocol(Protocol::UTF8SMTPA),
            "utf8smtps" => Kind::Protocol(Protocol::UTF8SMTPS),
            "utf8smtpsa" => Kind::Protocol(Protocol::UTF8SMTPSA),
            "utf8lmtp" => Kind::Protocol(Protocol::UTF8LMTP),
            "utf8lmtpa" => Kind::Protocol(Protocol::UTF8LMTPA),
            "utf8lmtps" => Kind::Protocol(Protocol::UTF8LMTPS),
            "utf8lmtpsa" => Kind::Protocol(Protocol::UTF8LMTPSA),
            "http" => Kind::Protocol(Protocol::HTTP),
            "httpu" => Kind::Protocol(Protocol::HTTP),
            "httprest" => Kind::Protocol(Protocol::HTTP),
            "https" => Kind::Protocol(Protocol::HTTPS),
            "imap" => Kind::Protocol(Protocol::IMAP),
            "pop3" => Kind::Protocol(Protocol::POP3),
            "local" => Kind::Protocol(Protocol::Local),
            "socket" => Kind::Protocol(Protocol::Local),
            "stdin" => Kind::Protocol(Protocol::Local),
            "ssl2" => Kind::Tls(TlsVersion::SSLv2),
            "sslv2" => Kind::Tls(TlsVersion::SSLv2),
            "ssl3" => Kind::Tls(TlsVersion::SSLv3),
            "sslv3" => Kind::Tls(TlsVersion::SSLv3),
            "tls1" => Kind::Tls(TlsVersion::TLSv1_0),
            "tlsv1" => Kind::Tls(TlsVersion::TLSv1_0),
            "tls10" => Kind::Tls(TlsVersion::TLSv1_0),
            "tlsv10" => Kind::Tls(TlsVersion::TLSv1_0),
            "tls11" => Kind::Tls(TlsVersion::TLSv1_1),
            "tlsv11" => Kind::Tls(TlsVersion::TLSv1_1),
            "tls12" => Kind::Tls(TlsVersion::TLSv1_2),
            "tlsv12" => Kind::Tls(TlsVersion::TLSv1_2),
            "tls13" => Kind::Tls(TlsVersion::TLSv1_3),
            "tlsv13" => Kind::Tls(TlsVersion::TLSv1_3),
            "dtls10" => Kind::Tls(TlsVersion::DTLSv1_0),
            "dtlsv10" => Kind::Tls(TlsVersion::DTLSv1_0),
            "dtls12" => Kind::Tls(TlsVersion::DTLSv1_2),
            "dtlsv12" => Kind::Tls(TlsVersion::DTLSv1_2),
            "dtls13" => Kind::Tls(TlsVersion::DTLSv1_3),
            "dtlsv13" => Kind::Tls(TlsVersion::DTLSv1_3),
        )
        .copied()
        .unwrap_or(Kind::Word)
    }

    fn decorated_keyword(text: &[u8], mask: u16) -> Kind {
        let first = text
            .iter()
            .find(|byte| byte.is_ascii_alphanumeric())
            .map(u8::to_ascii_lowercase);
        let candidate = if mask & DIGIT == 0 {
            matches!(first, Some(b'l' | b'e'))
        } else {
            matches!(first, Some(b't' | b'd'))
        };
        if candidate {
            Kind::decorated_lookup(text, mask)
        } else {
            Kind::Word
        }
    }

    fn decorated_lookup(text: &[u8], mask: u16) -> Kind {
        let letters = text
            .iter()
            .filter(|byte| byte.is_ascii_alphanumeric())
            .map(u8::to_ascii_lowercase);
        let mut alnum = [0u8; MAX_KEYWORD];
        let mut len = 0;
        for byte in letters {
            let Some(slot) = alnum.get_mut(len) else {
                return Kind::Word;
            };
            *slot = byte;
            len += 1;
        }
        let alnum = alnum.get(..len).unwrap_or_default();
        if mask & DIGIT == 0 {
            if mask & !(ALNUM | MINUS) != 0 {
                return Kind::Word;
            }
            match alnum {
                b"localesmtp" => Kind::Protocol(Protocol::ESMTP),
                b"localesmtps" | b"esmtptls" => Kind::Protocol(Protocol::ESMTPS),
                b"localbsmtp" => Kind::Protocol(Protocol::SMTP),
                _ => Kind::Word,
            }
        } else {
            let version = match alnum {
                b"tls10" => return Kind::Tls(TlsVersion::TLSv1_0),
                b"tls11" => return Kind::Tls(TlsVersion::TLSv1_1),
                b"tls12" => return Kind::Tls(TlsVersion::TLSv1_2),
                b"tls13" => return Kind::Tls(TlsVersion::TLSv1_3),
                b"tlsv10" => TlsVersion::TLSv1_0,
                b"tlsv11" => TlsVersion::TLSv1_1,
                b"tlsv12" => TlsVersion::TLSv1_2,
                b"tlsv13" => TlsVersion::TLSv1_3,
                b"dtls10" | b"dtlsv10" => TlsVersion::DTLSv1_0,
                b"dtls12" | b"dtlsv12" => TlsVersion::DTLSv1_2,
                b"dtls13" | b"dtlsv13" => TlsVersion::DTLSv1_3,
                _ => return Kind::Word,
            };
            if mask & MINUS == 0 {
                Kind::Tls(version)
            } else {
                Kind::Word
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Token {
    start: u32,
    end: u32,
    kind: Kind,
    mask: u16,
    colons: u8,
    flags: u8,
}

impl Token {
    fn separator(kind: Kind) -> Token {
        Token {
            start: 0,
            end: 0,
            kind,
            mask: 0,
            colons: 0,
            flags: 0,
        }
    }

    fn in_comment(&self) -> bool {
        self.flags & IN_COMMENT != 0
    }

    fn in_bracket(&self) -> bool {
        self.flags & IN_BRACKET != 0
    }

    fn range(&self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    fn text<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        bytes.get(self.range()).unwrap_or_default()
    }

    fn ip(&self, bytes: &[u8]) -> Option<IpAddr> {
        if self.kind != Kind::Word {
            return None;
        }
        let text = self.text(bytes);
        if self.colons == 0 {
            if self.mask & DOT == 0 || self.mask & !(DIGIT | DOT) != 0 {
                return None;
            }
            ipv4(text).map(IpAddr::V4)
        } else {
            if self.colons < 2
                || self.mask & (ALPHA | AT | OTHER | UNDERSCORE | PLUS | MINUS | UTF) != 0
            {
                return None;
            }
            let count = |class_bits: u16| {
                text.iter()
                    .filter(|&&byte| class(byte) & class_bits != 0)
                    .count()
            };
            let (hex, digits) = (count(HEX), count(DIGIT));
            let shaped = if self.mask & DOT == 0 {
                (1..=32).contains(&hex) || (1..=32).contains(&digits)
            } else {
                self.colons == 3 && hex == 4 && count(DOT) == 3 && (4..=12).contains(&digits)
            };
            if !shaped {
                return None;
            }
            std::str::from_utf8(text)
                .ok()?
                .parse::<Ipv6Addr>()
                .ok()
                .map(IpAddr::V6)
        }
    }

    fn integer(&self, bytes: &[u8]) -> Option<i64> {
        if self.kind != Kind::Word
            || self.colons != 0
            || self.mask & DIGIT == 0
            || self.mask & !(DIGIT | PLUS | MINUS) != 0
        {
            return None;
        }
        let text = self.text(bytes);
        let (negative, digits) = match text.split_first() {
            Some((b'-', digits)) => (true, digits),
            Some((b'+', digits)) => (false, digits),
            _ => (false, text),
        };
        if digits.is_empty() {
            return None;
        }
        digits.iter().try_fold(0i64, |value, &byte| {
            let digit = i64::from(byte.checked_sub(b'0').filter(|digit| *digit <= 9)?);
            let value = value.checked_mul(10)?;
            if negative {
                value.checked_sub(digit)
            } else {
                value.checked_add(digit)
            }
        })
    }

    fn is_email(&self, bytes: &[u8]) -> bool {
        self.kind == Kind::Word
            && self.mask & AT != 0
            && self.mask & LETTER != 0
            && self
                .text(bytes)
                .iter()
                .filter(|&&byte| byte == b'@')
                .count()
                == 1
    }

    fn is_domain(&self) -> bool {
        self.kind == Kind::Word
            && self.colons == 0
            && self.mask & LETTER != 0
            && self.mask & DOT != 0
            && self.mask & (AT | OTHER | UNDERSCORE | PLUS) == 0
    }

    fn is_cipher(&self, bytes: &[u8]) -> bool {
        if self.kind != Kind::Word
            || self.colons != 0
            || self.range().len() < MIN_CIPHER
            || self.mask & (LOWER | DOT | AT | PLUS | OTHER | UTF) != 0
            || self.mask & DIGIT == 0
            || self.mask & (UNDERSCORE | MINUS) == 0
        {
            return false;
        }
        let mut prefix = [0u8; 3];
        let mut letters = self
            .text(bytes)
            .iter()
            .filter(|byte| byte.is_ascii_alphanumeric());
        for slot in &mut prefix {
            match letters.next() {
                Some(byte) => *slot = byte.to_ascii_lowercase(),
                None => return false,
            }
        }
        CIPHER_PREFIXES.contains(&&prefix)
    }
}

fn ipv4(text: &[u8]) -> Option<Ipv4Addr> {
    let mut octets = [0u8; 4];
    let mut groups = text.split(|&byte| byte == b'.');
    for octet in &mut octets {
        let group = groups.next()?;
        if group.is_empty() || group.len() > 3 || (group.len() > 1 && group.first() == Some(&b'0'))
        {
            return None;
        }
        let value = group.iter().try_fold(0u16, |value, &byte| {
            byte.is_ascii_digit()
                .then(|| value * 10 + u16::from(byte - b'0'))
        })?;
        *octet = u8::try_from(value).ok()?;
    }
    groups.next().is_none().then_some(Ipv4Addr::from(octets))
}

struct Tokenizer<'a> {
    bytes: &'a [u8],
    pos: usize,
    pending: Option<Kind>,
    comment_depth: u32,
    in_bracket: bool,
    in_quote: bool,
    in_date: bool,
}

impl<'a> Tokenizer<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Tokenizer {
            bytes,
            pos: 0,
            pending: None,
            comment_depth: 0,
            in_bracket: false,
            in_quote: false,
            in_date: false,
        }
    }

    fn next(&mut self, skip_inert: bool) -> Option<Token> {
        loop {
            let token = match self.pending.take() {
                Some(kind) => Token::separator(kind),
                None => self.scan()?,
            };
            if !skip_inert || !token.kind.is_inert() {
                return Some(token);
            }
        }
    }

    fn scan(&mut self) -> Option<Token> {
        let rest = self.bytes.get(self.pos..).unwrap_or_default();
        let Some(skip) = rest.iter().position(|&byte| class(byte) != SPACE) else {
            self.pos = self.bytes.len();
            return None;
        };
        let word = rest.get(skip..).unwrap_or_default();
        let start = self.pos + skip;
        let flags = (u8::from(self.comment_depth > 0) * IN_COMMENT)
            | (u8::from(self.in_bracket) * IN_BRACKET);
        let in_date = self.in_date;
        let mut mask = 0;
        let mut colons = 0;
        let mut bytes = word.iter();
        let stop = loop {
            let Some(&byte) = bytes.next() else {
                break None;
            };
            let class = class(byte);
            if class & WORD != 0 {
                mask |= class;
            } else if class == COLON && !in_date && mask & !HEX_DIGIT == 0 && colons < MAX_COLONS {
                colons += 1;
            } else {
                break Some(byte);
            }
        };
        let consumed = word.len() - bytes.as_slice().len();
        self.pos = start + consumed;
        let len = match stop {
            Some(stop) => {
                if class(stop) != SPACE {
                    self.consume(stop);
                }
                consumed - 1
            }
            None => consumed,
        };
        if len == 0 {
            return self.pending.take().map(Token::separator);
        }
        let text = word.get(..len).unwrap_or_default();
        Some(Token {
            start: start as u32,
            end: (start + len) as u32,
            kind: Kind::of_word(text, mask, colons),
            mask,
            colons,
            flags,
        })
    }

    fn canonical_date(&mut self) -> Option<[i64; 7]> {
        if !self.in_date {
            return None;
        }
        let mut cursor = Cursor {
            rest: self.bytes.get(self.pos..).unwrap_or_default(),
        };
        let mut token = cursor.token();
        if is_day_name(token) {
            cursor.byte(b',');
            token = cursor.token();
        }
        let day = digits(token, 2)?;
        let Kind::Month(month) = Kind::keyword(cursor.token()) else {
            return None;
        };
        let year = digits(cursor.token(), 4)?;
        let hour = digits(cursor.token(), 2)?;
        cursor.byte(b':').then_some(())?;
        let minute = digits(cursor.token(), 2)?;
        cursor.byte(b':').then_some(())?;
        let second = digits(cursor.token(), 2)?;
        let zone = zone_offset(cursor.token())?;
        self.pos = self.bytes.len() - cursor.rest.len();
        Some([day, i64::from(month), year, hour, minute, second, zone])
    }

    fn consume(&mut self, byte: u8) {
        let kind = match byte {
            b'(' => {
                if !self.in_quote {
                    self.comment_depth = self.comment_depth.saturating_add(1);
                }
                Kind::ParenOpen
            }
            b')' => {
                if !self.in_quote {
                    self.comment_depth = self.comment_depth.saturating_sub(1);
                }
                Kind::ParenClose
            }
            b'[' => {
                if !self.in_quote {
                    self.in_bracket = true;
                }
                Kind::BracketOpen
            }
            b']' => {
                if !self.in_quote {
                    self.in_bracket = false;
                }
                Kind::BracketClose
            }
            b';' => {
                if self.comment_depth == 0 {
                    self.in_date = true;
                }
                Kind::Semicolon
            }
            b'"' => {
                self.in_quote = !self.in_quote;
                Kind::Quote
            }
            b'<' => Kind::AngleOpen,
            b'>' => Kind::AngleClose,
            b'=' => Kind::Equal,
            b'/' => Kind::Slash,
            b',' => Kind::Comma,
            b':' => Kind::Colon,
            _ => return,
        };
        self.pending = Some(kind);
    }
}

struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn skip_space(&mut self) {
        let spaces = self
            .rest
            .iter()
            .position(|&byte| class(byte) != SPACE)
            .unwrap_or(self.rest.len());
        self.rest = self.rest.get(spaces..).unwrap_or_default();
    }

    fn token(&mut self) -> &'a [u8] {
        self.skip_space();
        let len = self
            .rest
            .iter()
            .position(|&byte| class(byte) & WORD == 0)
            .unwrap_or(self.rest.len());
        let (token, rest) = self.rest.split_at(len);
        self.rest = rest;
        token
    }

    fn byte(&mut self, byte: u8) -> bool {
        self.skip_space();
        match self.rest.split_first() {
            Some((&first, rest)) if first == byte => {
                self.rest = rest;
                true
            }
            _ => false,
        }
    }
}

fn is_day_name(token: &[u8]) -> bool {
    token.len() == 3
        && DAY_NAMES
            .iter()
            .any(|name| token.eq_ignore_ascii_case(*name))
}

fn digits(token: &[u8], max: usize) -> Option<i64> {
    if token.is_empty() || token.len() > max {
        return None;
    }
    token.iter().try_fold(0, |value, &byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + i64::from(byte - b'0'))
    })
}

fn zone_offset(token: &[u8]) -> Option<i64> {
    let (&sign, digits_part) = token.split_first()?;
    let value = digits(digits_part, 4).filter(|_| digits_part.len() == 4)?;
    match sign {
        b'+' => Some(value),
        b'-' => Some(-value),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Other,
    From,
    Date,
}

struct DateParts {
    parts: [i64; 7],
    len: usize,
}

impl DateParts {
    fn push(&mut self, value: i64) {
        if let Some(part) = self.parts.get_mut(self.len) {
            *part = value;
            self.len += 1;
        }
    }

    fn build(&self) -> Option<DateTime> {
        let [day, month, year, hour, minute, second, zone] = self.parts;
        if second == UNSET {
            return None;
        }
        let (zone, is_plus) = match zone {
            UNSET => (0, false),
            zone if zone < 0 => (zone.wrapping_abs(), false),
            zone => (zone, true),
        };
        Some(DateTime {
            year: match year {
                0..=49 => year + 2000,
                50..=99 => year + 1900,
                _ => year,
            } as u16,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            tz_hour: (zone / 100) as u8,
            tz_minute: (zone % 100) as u8,
            tz_before_gmt: !is_plus,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Nothing,
    From,
    By,
    Helo,
    Id,
    Via,
    Ident,
    For,
    With,
}

impl Expect {
    fn skips(self, kind: Kind) -> bool {
        match self {
            Expect::From => kind == Kind::BracketOpen,
            Expect::By => matches!(kind, Kind::BracketOpen | Kind::AngleOpen),
            Expect::Helo => matches!(kind, Kind::Equal | Kind::BracketOpen | Kind::Colon),
            Expect::Id | Expect::Ident => matches!(
                kind,
                Kind::Equal | Kind::AngleOpen | Kind::BracketOpen | Kind::Colon
            ),
            Expect::Via => kind == Kind::Equal,
            Expect::For => matches!(kind, Kind::Equal | Kind::AngleOpen),
            Expect::Nothing | Expect::With => false,
        }
    }
}

struct Grammar<'c, 'a> {
    ctx: &'c FieldCtx<'a>,
    bytes: &'a [u8],
    base: usize,
    tokens: Tokenizer<'a>,
    entry: ReceivedEntry,
    state: State,
    expect: Expect,
    date: DateParts,
}

impl<'c, 'a> Grammar<'c, 'a> {
    fn new(ctx: &'c FieldCtx<'a>, value: Range<usize>) -> Self {
        let bytes = ctx.bytes(value.clone());
        let bytes = bytes.get(..MAX_VALUE).unwrap_or(bytes);
        Grammar {
            ctx,
            bytes,
            base: value.start,
            tokens: Tokenizer::new(bytes),
            entry: ReceivedEntry::EMPTY,
            state: State::Other,
            expect: Expect::Nothing,
            date: DateParts {
                parts: [UNSET; 7],
                len: 0,
            },
        }
    }

    fn run(mut self) -> ReceivedEntry {
        while let Some(token) = self.tokens.next(self.expect == Expect::Nothing) {
            if self.expect == Expect::Nothing || !self.expected(&token) {
                self.dispatch(&token);
            }
        }
        self.entry.date = self.date.build();
        self.entry
    }

    fn expected(&mut self, token: &Token) -> bool {
        let expect = self.expect;
        if expect.skips(token.kind) {
            return true;
        }
        match expect {
            Expect::With => match token.kind {
                Kind::Protocol(protocol) => self.entry.with = Some(protocol),
                Kind::Semicolon
                | Kind::Tls(_)
                | Kind::By
                | Kind::For
                | Kind::From
                | Kind::Id
                | Kind::Via
                | Kind::With => {
                    self.expect = Expect::Nothing;
                    return false;
                }
                _ => return true,
            },
            Expect::For => {
                if !token.is_email(self.bytes) {
                    self.expect = Expect::Nothing;
                    return false;
                }
                self.entry.for_ = self.text(token.range());
            }
            _ if token.kind.is_separator() => {
                self.expect = Expect::Nothing;
                return false;
            }
            Expect::From => self.entry.from = self.host(token),
            Expect::By => self.entry.by = self.host(token),
            Expect::Helo => self.entry.helo = self.host(token),
            Expect::Id => self.entry.id = self.text(token.range()),
            Expect::Via => self.entry.via = self.text(token.range()),
            Expect::Ident => self.entry.ident = self.text(token.range()),
            Expect::Nothing => return false,
        }
        self.expect = Expect::Nothing;
        true
    }

    fn dispatch(&mut self, token: &Token) {
        let in_comment = token.in_comment();
        match token.kind {
            Kind::From if self.entry.from == HostEntry::None => {
                self.clause(Expect::From, State::From);
            }
            Kind::By if !in_comment => self.clause(Expect::By, State::Other),
            Kind::For if !in_comment => self.clause(Expect::For, State::Other),
            Kind::Semicolon => {
                self.state = State::Date;
                if let Some(parts) = self.tokens.canonical_date() {
                    parts.into_iter().for_each(|part| self.date.push(part));
                }
            }
            Kind::Id if !in_comment => self.clause(Expect::Id, State::Other),
            Kind::With if !in_comment => self.clause(Expect::With, State::Other),
            Kind::Via if !in_comment => self.clause(Expect::Via, State::Other),
            Kind::Ident if in_comment => self.expect = Expect::Ident,
            Kind::Greeting(greeting) if self.state == State::From && in_comment => {
                self.entry.helo_cmd = Some(greeting);
                self.expect = Expect::Helo;
            }
            Kind::Month(month) if self.state == State::Date => {
                self.date.push(i64::from(month));
            }
            Kind::Tls(version) if in_comment && self.entry.tls_version.is_none() => {
                self.entry.tls_version = Some(version);
            }
            Kind::Word => self.word(token),
            _ => (),
        }
    }

    fn clause(&mut self, expect: Expect, state: State) {
        self.expect = expect;
        self.state = state;
    }

    fn word(&mut self, token: &Token) {
        let in_comment = token.in_comment();
        match self.state {
            State::From => {
                if let Some(ip) = token.ip(self.bytes) {
                    if token.in_bracket() || (in_comment && self.entry.from_ip.is_none()) {
                        self.entry.from_ip = Some(ip);
                    }
                    return;
                }
                if token.is_domain() {
                    if in_comment {
                        self.entry.from_iprev = self.text(token.range());
                    }
                    return;
                }
                if token.is_email(self.bytes) {
                    let range = token.range();
                    let end = range.end - usize::from(token.text(self.bytes).ends_with(b"@"));
                    self.entry.ident = self.text(range.start..end);
                    return;
                }
            }
            State::Date => {
                if let Some(value) = token.integer(self.bytes) {
                    self.date.push(value);
                    return;
                }
            }
            State::Other => (),
        }
        if (in_comment || self.entry.tls_cipher.is_none()) && token.is_cipher(self.bytes) {
            self.entry.tls_cipher = self.text(token.range());
        }
    }

    fn host(&self, token: &Token) -> HostEntry {
        match token.ip(self.bytes) {
            Some(ip) => HostEntry::Ip(ip),
            None => HostEntry::Name(self.text(token.range())),
        }
    }

    fn text(&self, range: Range<usize>) -> Str {
        self.ctx
            .try_borrow(self.base + range.start..self.base + range.end)
            .unwrap_or(Str::EMPTY)
    }
}

#[cfg(test)]
mod tests {
    use crate::{HeaderForm, Host, MessageParser, Received, fields::tests::load_tests};
    use serde_json::{Map, Value as Json, json};

    fn host_json(host: Host<'_>) -> Json {
        match host {
            Host::Name(name) => json!({ "Name": name }),
            Host::IpAddr(ip) => json!({ "IpAddr": ip.to_string() }),
        }
    }

    fn received_json(received: Received<'_>) -> Json {
        let debug = |value: &dyn std::fmt::Debug| json!(format!("{value:?}"));
        let fields = [
            ("from", received.from().map(host_json)),
            (
                "from_ip",
                received.from_ip().map(|ip| json!(ip.to_string())),
            ),
            ("from_iprev", received.from_iprev().map(|text| json!(text))),
            ("by", received.by().map(host_json)),
            ("for_", received.for_().map(|text| json!(text))),
            ("with", received.with().map(|with| debug(&with))),
            (
                "tls_version",
                received.tls_version().map(|version| debug(&version)),
            ),
            ("tls_cipher", received.tls_cipher().map(|text| json!(text))),
            ("id", received.id().map(|text| json!(text))),
            ("ident", received.ident().map(|text| json!(text))),
            ("helo", received.helo().map(host_json)),
            ("helo_cmd", received.helo_cmd().map(|cmd| debug(&cmd))),
            ("via", received.via().map(|text| json!(text))),
            (
                "date",
                received.date().map(|date| {
                    json!({
                        "year": date.year,
                        "month": date.month,
                        "day": date.day,
                        "hour": date.hour,
                        "minute": date.minute,
                        "second": date.second,
                        "tz_before_gmt": date.tz_before_gmt,
                        "tz_hour": date.tz_hour,
                        "tz_minute": date.tz_minute,
                    })
                }),
            ),
        ];
        Json::Object(
            fields
                .into_iter()
                .filter_map(|(name, value)| Some((name.to_string(), value?)))
                .collect::<Map<_, _>>(),
        )
    }

    fn parse(header: &[u8]) -> Json {
        HeaderForm::Received
            .parse(header)
            .value()
            .as_received()
            .map_or(Json::Null, received_json)
    }

    fn without_nulls(expected: Json) -> Json {
        match expected {
            Json::Object(fields) => Json::Object(
                fields
                    .into_iter()
                    .filter(|(_, value)| !value.is_null())
                    .collect(),
            ),
            other => other,
        }
    }

    #[test]
    fn received_fixtures() {
        let tests = load_tests("received.json");
        assert_eq!(tests.len(), 202);
        for (header, expected) in tests {
            assert_eq!(
                parse(header.as_bytes()),
                without_nulls(expected),
                "{header:?}"
            );
        }
    }

    #[test]
    fn received_edge_cases() {
        assert_eq!(
            parse(b"from \xff\xfe by y\n"),
            json!({ "from": { "Name": "" }, "by": { "Name": "y" } })
        );
    }

    #[test]
    fn ipv4_matches_std() {
        const GROUPS: [&str; 22] = [
            "", "0", "00", "01", "1", "9", "10", "99", "100", "199", "200", "249", "250", "255",
            "256", "299", "300", "999", "0000", "1000", "025", "x",
        ];
        let mut text = String::with_capacity(32);
        for a in GROUPS {
            for b in GROUPS {
                for c in GROUPS {
                    for d in GROUPS {
                        for tail in ["", ".", ".1", "..1"] {
                            text.clear();
                            text.push_str(&[a, b, c, d].join("."));
                            text.push_str(tail);
                            assert_eq!(
                                super::ipv4(text.as_bytes()),
                                text.parse::<std::net::Ipv4Addr>().ok(),
                                "{text:?}"
                            );
                        }
                    }
                }
            }
        }
        for text in [
            "1.2.3", "1.2", "1", "", ".", "...", "1..2.3", "+1.2.3.4", "1.2.3.4 ",
        ] {
            assert_eq!(super::ipv4(text.as_bytes()), text.parse().ok(), "{text:?}");
        }
    }

    #[test]
    fn keyword_table() {
        for (keyword, kind) in super::KEYWORDS {
            assert_eq!(super::Kind::keyword(keyword), kind, "{keyword:?}");
            let upper = keyword.to_ascii_uppercase();
            assert_eq!(super::Kind::keyword(&upper), kind, "{upper:?}");
        }
    }

    #[test]
    fn received_truncated_fold() {
        for message in [
            &b"Received:\n\t"[..],
            b"Received:\r\n ",
            b"Received: x\r\n ",
            b"Received: x\r\n\t",
            b"Received: x\n ",
            b"Received: x\n\t",
            b"Received: x\r\n  ",
            b"Received: x\r\n \t",
            b"Received: x\r\n \t ",
            b"Received: x\r\n \r",
            b"Received: x\r\n \r\n",
            b"Received: x\r\n \r\nbody",
            b"Received: x\r\n y\r\n ",
            b"Received: x;\r\n ",
            b"To: a@b\r\nReceived: x\r\n ",
            b"Received: from x (y)\r\n\t",
            b"Received: from x by y;\r\n\t",
            b"Received: ",
            b"Received:",
            b"Received: x",
        ] {
            let parser = MessageParser::default();
            let _ = parser.parse(message);
            let _ = parser.parse_headers(message);
            let _ = parse(message.get(9..).unwrap_or_default());
        }
    }
}
