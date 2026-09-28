/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::str;

const MAX_PREFIX_LEN: usize = 40;
const MAX_GENERIC_PREFIX_CHARS: usize = 4;
const LOWERCASE_BUFFER_LEN: usize = 64;
const URL_SCHEME_SEPARATOR: &str = "://";

fn is_re_prefix(prefix: &str) -> bool {
    hashify::set! {prefix.as_bytes(),
        "re",
        "res",
        "resp",
        "resposta",
        "respuesta",
        "rsp",
        "ref",
        "réf",
        "rép",
        "rep",
        "réponse",
        "reponse",
        "aw",
        "antw",
        "antwort",
        "antwoord",
        "sv",
        "svar",
        "svara",
        "vast",
        "vastaus",
        "vastus",
        "vá",
        "va",
        "válasz",
        "valasz",
        "r",
        "ri",
        "rif",
        "odp",
        "odpowiedź",
        "odpowiedz",
        "odpov",
        "odg",
        "odgovor",
        "απ",
        "σχετ",
        "απάντηση",
        "ynt",
        "yn",
        "yan",
        "yanıt",
        "yanit",
        "atb",
        "atb.",
        "atbilde",
        "ateb",
        "ats",
        "ats.",
        "atsakymas",
        "bls",
        "balas",
        "השב",
        "תשובה",
        "رد",
        "پاسخ",
        "отв",
        "ответ",
        "відп",
        "відповідь",
        "отг",
        "отговор",
        "回复",
        "回覆",
        "答复",
        "答覆",
        "返信",
        "회신",
        "답장",
        "ตอบกลับ",
        "பதில்",
        "trả lời",
    }
}

fn is_fwd_prefix(prefix: &str) -> bool {
    hashify::set! {prefix.as_bytes(),
        "fwd",
        "fw",
        "rv",
        "reenviado",
        "enc",
        "encaminhado",
        "tr",
        "transfert",
        "wg",
        "weitergeleitet",
        "doorst",
        "doorgestuurd",
        "vs",
        "videresendt",
        "vb",
        "vidarebefordrat",
        "vl",
        "välitetty",
        "ed",
        "edastatud",
        "fs",
        "framsenda",
        "i",
        "inoltro",
        "inoltrato",
        "pd",
        "podaj dalej",
        "przekazane",
        "prosl",
        "proslijeđeno",
        "πρθ",
        "προωθ",
        "προωθημένο",
        "i\u{307}lt",
        "ilt",
        "i\u{307}let",
        "ilet",
        "i\u{307}letilen",
        "iletilen",
        "yml",
        "ymlaen",
        "pārs",
        "pārs.",
        "pārsūtīts",
        "persiųsta",
        "trs",
        "terusan",
        "teruskan",
        "továbbítás",
        "הועבר",
        "העברה",
        "إعادة توجيه",
        "перес",
        "переслано",
        "пересылка",
        "пре",
        "препратено",
        "转发",
        "转寄",
        "轉寄",
        "転送",
        "전달",
        "ส่งต่อ",
        "முன்னனுப்பு",
        "chuyển tiếp",
    }
}

fn is_known_prefix(prefix: &str) -> bool {
    is_re_prefix(prefix) || is_fwd_prefix(prefix)
}

fn is_generic_prefix(prefix: &str) -> bool {
    let mut char_count = 0;

    for ch in prefix.chars() {
        if !ch.is_alphabetic() {
            return false;
        }
        char_count += 1;
        if char_count > MAX_GENERIC_PREFIX_CHARS {
            return false;
        }
    }

    char_count > 0
}

fn with_lowercase<T>(text: &str, f: impl FnOnce(&str) -> T) -> T {
    let mut buffer = [0u8; LOWERCASE_BUFFER_LEN];
    match buffer.get_mut(..text.len()) {
        Some(lower) if text.is_ascii() => {
            lower.copy_from_slice(text.as_bytes());
            lower.make_ascii_lowercase();
            f(str::from_utf8(lower).unwrap_or_default())
        }
        _ => f(&text.to_lowercase()),
    }
}

fn is_prefix(text: &str, token_start: usize, token_end: usize, separator: usize) -> bool {
    let token = text.get(token_start..token_end).unwrap_or_default();
    let is_token_prefix = with_lowercase(token, |token| {
        if is_known_prefix(token) {
            Some(true)
        } else if token_end == separator {
            Some(
                !text
                    .get(separator..)
                    .unwrap_or_default()
                    .starts_with(URL_SCHEME_SEPARATOR)
                    && is_generic_prefix(token),
            )
        } else {
            None
        }
    });
    is_token_prefix.unwrap_or_else(|| {
        let span = text
            .get(token_start..separator)
            .unwrap_or_default()
            .trim_end();
        span.len() <= MAX_PREFIX_LEN && with_lowercase(span, is_known_prefix)
    })
}

/// The thread name of a subject: the subject without reply and forward
/// prefixes (`Re:`, `Fwd:` and their translations, with counters such as
/// `Re[2]:`), bracketed list tags and trailing `(fwd)` markers, as used to
/// group messages into threads. Borrows from `text`.
///
/// ```
/// use mail_parser::thread_name;
///
/// assert_eq!(thread_name("Re: Fwd: [list] Lunch on Friday"), "Lunch on Friday");
/// ```
pub fn thread_name(text: &str) -> &str {
    let mut token_start = 0;
    let mut token_end = 0;

    let mut thread_name_start = 0;
    let mut fwd_start = 0;
    let mut fwd_end = 0;
    let mut last_blob_end = 0;

    let mut in_blob = false;
    let mut in_blob_ignore = false;
    let mut seen_header = false;
    let mut seen_blob_header = false;
    let mut token_found = false;

    for (pos, ch) in text.char_indices() {
        match ch {
            '[' => {
                if !in_blob {
                    if token_found {
                        if token_end == 0 {
                            token_end = pos;
                        }
                        if is_prefix(text, token_start, token_end, pos) {
                            seen_header = true;
                        } else {
                            break;
                        }
                    }
                    token_found = false;
                    in_blob = true;
                } else {
                    break;
                }
            }
            ']' if in_blob => {
                if seen_blob_header && token_found {
                    fwd_start = token_start;
                    fwd_end = pos;
                }
                if !seen_header {
                    last_blob_end = pos + 1;
                }
                in_blob = false;
                token_found = false;
                seen_blob_header = false;
                in_blob_ignore = false;
            }
            ':' if !in_blob => {
                if (seen_header && token_found) || (!seen_header && !token_found) {
                    break;
                } else if !seen_header {
                    if token_end == 0 {
                        token_end = pos;
                    }
                    if !is_prefix(text, token_start, token_end, pos) {
                        break;
                    }
                } else {
                    seen_header = false;
                }
                thread_name_start = pos + 1;
                token_found = false;
            }
            ':' if in_blob && !in_blob_ignore => {
                if token_end == 0 {
                    token_end = pos;
                }

                let token = text.get(token_start..token_end).unwrap_or_default();
                with_lowercase(token, |prefix| {
                    if is_fwd_prefix(prefix) {
                        token_found = false;
                        seen_blob_header = true;
                    } else if seen_blob_header && is_re_prefix(prefix) {
                        token_found = false;
                    } else {
                        in_blob_ignore = true;
                    }
                });
            }
            _ if ch.is_whitespace() => {
                if token_end == 0 {
                    token_end = pos;
                }
            }
            _ => {
                if !token_found {
                    token_start = pos;
                    token_end = 0;
                    token_found = true;
                } else if !in_blob && pos - token_start > MAX_PREFIX_LEN {
                    break;
                }
            }
        }
    }

    if last_blob_end > thread_name_start
        || (fwd_start > 0 && last_blob_end > fwd_start && fwd_start > thread_name_start)
    {
        let result = trim_trailing_fwd(text.get(last_blob_end..).unwrap_or_default());
        if !result.is_empty() {
            return result;
        }
    }

    if fwd_start > 0 && thread_name_start < fwd_start {
        let result = trim_trailing_fwd(text.get(fwd_start..fwd_end).unwrap_or_default());
        if !result.is_empty() {
            return result;
        }
    }

    trim_trailing_fwd(text.get(thread_name_start..).unwrap_or_default())
}

fn trim_trailing_fwd(text: &str) -> &str {
    let mut in_parentheses = false;
    let mut trim_end = true;

    let mut text_start = 0;
    let mut text_end = text.len();
    let mut fwd_end = 0;

    for (pos, ch) in text.char_indices().rev() {
        let end_found = match ch {
            '(' => {
                if in_parentheses {
                    in_parentheses = false;
                    if fwd_end - pos > 2
                        && with_lowercase(
                            text.get(pos + 1..fwd_end).unwrap_or_default(),
                            is_fwd_prefix,
                        )
                    {
                        text_end = pos;
                        trim_end = true;
                        continue;
                    }
                }
                true
            }
            ')' => {
                if in_parentheses {
                    true
                } else {
                    in_parentheses = true;
                    fwd_end = pos;
                    false
                }
            }
            _ if ch.is_whitespace() => {
                if trim_end {
                    text_end = pos;
                }
                continue;
            }
            _ => !in_parentheses,
        };

        if end_found {
            let start = text
                .char_indices()
                .find(|(_, ch)| !ch.is_whitespace())
                .map_or(pos, |(start, _)| start);
            return text.get(start..text_end).unwrap_or_default();
        }

        trim_end = false;
        text_start = pos;
    }

    text.get(text_start..text_end).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{thread_name, trim_trailing_fwd};

    #[test]
    fn parse_thread_name() {
        let tests = [
            ("re: hello", "hello"),
            ("re:re: hello", "hello"),
            ("re:fwd: hello", "hello"),
            ("fwd[5]:re[5]: hello", "hello"),
            ("fwd[99]:  re[40]: hello", "hello"),
            (": hello", ": hello"),
            ("re:: hello", ": hello"),
            ("[10] hello", "hello"),
            ("fwd[a]: hello", "hello"),
            ("re:", ""),
            ("re::", ":"),
            ("", ""),
            (" ", ""),
            ("回复: 轉寄: 轉寄", "轉寄"),
            ("aw[50]: wg: aw[1]: hallo", "hallo"),
            ("res: rv: enc: továbbítás: ", ""),
            ("[fwd: hello world]", "hello world"),
            ("re: enc: re[5]: [fwd: hello world]", "hello world"),
            ("[fwd: re: fw: hello world]", "hello world"),
            ("[fwd: hello world]: another text", ": another text"),
            ("[fwd: re: fwd:] another text", "another text"),
            ("[hello world]", "[hello world]"),
            ("re: fwd[9]: [hello world]", "[hello world]"),
            ("[mailing-list] hello world", "hello world"),
            ("[mailing-list] re: hello world", "hello world"),
            ("[mailing-list] wg[8]:re:  hello world", "hello world"),
            ("hello [world]", "hello [world]"),
            (" [hello] [world] ", "[hello] [world]"),
            ("[mailing-list] hello [world]", "hello [world]"),
            ("[hello [world]", "[hello [world]"),
            ("[]hello [world]", "hello [world]"),
            ("[fwd: re: re:] fwd[6]:re:  fw:", ""),
            ("[fwd hello] world hello", "world hello"),
            ("[fwd: مرحبا بالعالم]", "مرحبا بالعالم"),
            ("[fwd: hello world] مرحبا بالعالم", "مرحبا بالعالم"),
            ("  hello world  ", "hello world"),
            (
                "[mailing-list] wg[8]:re:  hello world (fwd)(fwd)",
                "hello world",
            ),
            ("[fwd: re: fw: hello world (fwd)]", "hello world"),
            (
                "res: rv: enc: továbbítás: hello world (doorst)",
                "hello world",
            ),
            ("[fwd: re: re: (fwd)] fwd[6]:re:  fw: (fwd)", ""),
        ];

        for (input, expected) in tests {
            assert_eq!(thread_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn parse_unknown_prefix() {
        let tests = [
            ("z: hello", "hello"),
            ("yn: merhaba", "merhaba"),
            ("yan: merhaba", "merhaba"),
            ("ilt: merhaba", "merhaba"),
            ("Ynt: Re: Yan: merhaba", "merhaba"),
            ("ans: hello", "hello"),
            ("ΑΠΝΤ: γεια", "γεια"),
            ("meeting: hello", "meeting: hello"),
            ("urgent: hello", "urgent: hello"),
            ("bug 123: hello", "bug 123: hello"),
            ("12:30 meeting", "12:30 meeting"),
            ("no re: foobar", "no re: foobar"),
            ("re: no re: foobar", "no re: foobar"),
            ("re-check: hello", "re-check: hello"),
            ("q3: results", "q3: results"),
            ("http://example.com/page", "http://example.com/page"),
            ("ftp://example.com", "ftp://example.com"),
            ("note: remember this", "remember this"),
            ("hi: there", "there"),
        ];

        for (input, expected) in tests {
            assert_eq!(thread_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn parse_localized_prefix() {
        let tests = [
            ("Re: hello", "hello"),
            ("RES: olá", "olá"),
            ("Resposta: olá", "olá"),
            ("RV: hola", "hola"),
            ("Respuesta: hola", "hola"),
            ("ENC: olá", "olá"),
            ("Encaminhado: olá", "olá"),
            ("RÉF: bonjour", "bonjour"),
            ("Rép: bonjour", "bonjour"),
            ("TR: bonjour", "bonjour"),
            ("Transfert: bonjour", "bonjour"),
            ("AW: hallo", "hallo"),
            ("Antwort: hallo", "hallo"),
            ("WG: hallo", "hallo"),
            ("Weitergeleitet: hallo", "hallo"),
            ("Antw: hallo", "hallo"),
            ("Doorst: hallo", "hallo"),
            ("Doorgestuurd: hallo", "hallo"),
            ("SV: hej", "hej"),
            ("Svar: hej", "hej"),
            ("VS: hej", "hej"),
            ("Videresendt: hej", "hej"),
            ("VB: hej", "hej"),
            ("Vidarebefordrat: hej", "hej"),
            ("VS: moi", "moi"),
            ("Vastaus: moi", "moi"),
            ("VL: moi", "moi"),
            ("Välitetty: moi", "moi"),
            ("ED: tere", "tere"),
            ("Edastatud: tere", "tere"),
            ("FS: halló", "halló"),
            ("Framsenda: halló", "halló"),
            ("Vá: szia", "szia"),
            ("Válasz: szia", "szia"),
            ("Továbbítás: szia", "szia"),
            ("R: ciao", "ciao"),
            ("RIF: ciao", "ciao"),
            ("I: ciao", "ciao"),
            ("Inoltro: ciao", "ciao"),
            ("Odp: cześć", "cześć"),
            ("Odpowiedź: cześć", "cześć"),
            ("PD: cześć", "cześć"),
            ("Podaj dalej: cześć", "cześć"),
            ("Odg: bok", "bok"),
            ("Odgovor: bok", "bok"),
            ("ΑΠ: γεια", "γεια"),
            ("ΣΧΕΤ: γεια", "γεια"),
            ("Απάντηση: γεια", "γεια"),
            ("ΠΡΘ: γεια", "γεια"),
            ("Προωθημένο: γεια", "γεια"),
            ("YNT: merhaba", "merhaba"),
            ("Yanıt: merhaba", "merhaba"),
            ("İLT: merhaba", "merhaba"),
            ("İletilen: merhaba", "merhaba"),
            ("Atb: sveiki", "sveiki"),
            ("Atbilde: sveiki", "sveiki"),
            ("Pārs: sveiki", "sveiki"),
            ("Pārsūtīts: sveiki", "sveiki"),
            ("Ats: labas", "labas"),
            ("Atsakymas: labas", "labas"),
            ("Persiųsta: labas", "labas"),
            ("ATB: helo", "helo"),
            ("Ateb: helo", "helo"),
            ("YML: helo", "helo"),
            ("Ymlaen: helo", "helo"),
            ("BLS: halo", "halo"),
            ("Balas: halo", "halo"),
            ("TRS: halo", "halo"),
            ("Teruskan: halo", "halo"),
            ("השב: שלום", "שלום"),
            ("תשובה: שלום", "שלום"),
            ("הועבר: שלום", "שלום"),
            ("העברה: שלום", "שלום"),
            ("رد: مرحبا", "مرحبا"),
            ("إعادة توجيه: مرحبا", "مرحبا"),
            ("پاسخ: سلام", "سلام"),
            ("Отв: привет", "привет"),
            ("Ответ: привет", "привет"),
            ("Пересылка: привет", "привет"),
            ("Переслано: привет", "привет"),
            ("Відп: привіт", "привіт"),
            ("Відповідь: привіт", "привіт"),
            ("Отг: здравей", "здравей"),
            ("Препратено: здравей", "здравей"),
            ("回复: 你好", "你好"),
            ("答复: 你好", "你好"),
            ("转发: 你好", "你好"),
            ("回覆: 你好", "你好"),
            ("轉寄: 你好", "你好"),
            ("返信: こんにちは", "こんにちは"),
            ("転送: こんにちは", "こんにちは"),
            ("회신: 안녕하세요", "안녕하세요"),
            ("답장: 안녕하세요", "안녕하세요"),
            ("전달: 안녕하세요", "안녕하세요"),
            ("ตอบกลับ: สวัสดี", "สวัสดี"),
            ("ส่งต่อ: สวัสดี", "สวัสดี"),
            ("பதில்: வணக்கம்", "வணக்கம்"),
            ("முன்னனுப்பு: வணக்கம்", "வணக்கம்"),
            ("Trả lời: xin chào", "xin chào"),
            ("Chuyển tiếp: xin chào", "xin chào"),
        ];

        for (input, expected) in tests {
            assert_eq!(thread_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn parse_mixed_locale_chain() {
        let tests = [
            ("Ynt: Re: Ynt: teklif", "teklif"),
            ("Re: Ynt: Re: teklif", "teklif"),
            ("AW: SV: Re: VS: hallo", "hallo"),
            (
                "Fwd: Re: Sv: Re: SV: vms rename Unix mode fixes",
                "vms rename Unix mode fixes",
            ),
            (
                "Re: RE: Re: Perl_peep recursion exceeds",
                "Perl_peep recursion exceeds",
            ),
            ("İlt: Ynt: rapor", "rapor"),
            ("回复: Re: 转发: 你好", "你好"),
            ("Odp: Re: Odp: cześć", "cześć"),
        ];

        for (input, expected) in tests {
            assert_eq!(thread_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn parse_cyrus_fixtures() {
        let tests = [
            ("understanding merge history", "understanding merge history"),
            (
                "Re: Alias of constant passed to sub",
                "Alias of constant passed to sub",
            ),
            (
                "[PATCH] merging make_ext and make_ext_cross",
                "merging make_ext and make_ext_cross",
            ),
            (
                "Re: [PATCH] Parallel testing conflict",
                "Parallel testing conflict",
            ),
            (
                "Re: [PATCH] Fwd: deprecate UNIVERSAL->import",
                "deprecate UNIVERSAL->import",
            ),
            ("Re[2]: another reply", "another reply"),
            ("Re[peat]: another reply", "another reply"),
            ("non\u{a0}breaking space", "non\u{a0}breaking space"),
            ("回复: test no ascii", "test no ascii"),
            ("re:\u{a0}non breaking space", "non breaking space"),
            ("\nfoo\rbar \tbaz ", "foo\rbar \tbaz"),
            (
                "how about the weather [SEC=UNOFFICIAL]",
                "how about the weather [SEC=UNOFFICIAL]",
            ),
            ("unmatched left] foobar", "unmatched left] foobar"),
        ];

        for (input, expected) in tests {
            assert_eq!(thread_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn parse_trail_fwd() {
        let tests = [
            ("hello (fwd)", "hello"),
            (" hello (fwd)(fwd)", "hello"),
            ("hello (wg) (fwd) (fwd)", "hello"),
            ("(fwd)(fwd)", ""),
            ("(fwd)hello(fwd)", "(fwd)hello"),
            ("  hello  ", "hello"),
            ("  hello world   ", "hello world"),
            ("", ""),
            ("    ", ""),
            ("hello ()(fwd)", "hello ()"),
            ("(hello)", "(hello)"),
            ("hello () (fwd) ()(fwd)", "hello () (fwd) ()"),
            (")(", ")("),
            (" 你好世界(fwd) ", "你好世界"),
            ("你好世界 (轉寄)", "你好世界"),
            ("merhaba (i\u{307}letilen)", "merhaba"),
            ("hello(fwd", "hello(fwd"),
            ("hello(fwd))", "hello(fwd))"),
        ];

        for (input, expected) in tests {
            assert_eq!(trim_trailing_fwd(input), expected, "{input:?}");
        }
    }
}
