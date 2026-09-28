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
    use crate::fields::tests::load_text_pairs;

    #[test]
    fn thread_name_fixtures() {
        let tests = load_text_pairs("thread_name.json");
        assert_eq!(tests.len(), 180);
        for (input, expected) in tests {
            assert_eq!(thread_name(&input), expected, "{input:?}");
        }
    }

    #[test]
    fn trailing_fwd_fixtures() {
        let tests = load_text_pairs("trailing_fwd.json");
        assert_eq!(tests.len(), 18);
        for (input, expected) in tests {
            assert_eq!(trim_trailing_fwd(&input), expected, "{input:?}");
        }
    }
}
