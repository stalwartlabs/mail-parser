use crate::{char_prefix, debug, valid};
use mail_parser::{MessagePart, PartKind, strip_charset_meta};

const PREFIX_CHARS: [usize; 7] = [0, 1, 2, 3, 7, 64, 1_000];
const PROBE_CHARS: usize = 8;
const APPENDED: &str = "appended";

pub fn check(part: MessagePart<'_>) {
    let decoded = part.decoded();
    assert_eq!(part.decoded_len(), decoded.len(), "decoded_len");
    let (checked, problems) = part.decoded_checked();
    assert_eq!(checked, decoded, "decoded_checked");
    assert_eq!(
        problems.names().count(),
        problems.bits().count_ones() as usize
    );
    debug(&problems);
    let mut appended = APPENDED.as_bytes().to_vec();
    part.decode_into(&mut appended);
    assert_eq!(
        appended.strip_prefix(APPENDED.as_bytes()),
        Some(decoded.as_ref()),
        "decode_into"
    );
    let Some(text) = part.text() else {
        assert!(!part.is_text());
        assert!(part.text_checked().is_none());
        assert!(part.text_prefix(PROBE_CHARS).is_none());
        assert!(!part.text_into(&mut String::new()));
        assert!(part.html_utf8().is_none());
        return;
    };
    valid(&text);
    assert!(part.is_text());
    let (checked, problems) = part.text_checked().expect("a text part has checked text");
    assert_eq!(checked, text, "text_checked");
    debug(&problems);
    let mut appended = String::from(APPENDED);
    assert!(part.text_into(&mut appended));
    assert_eq!(
        appended.strip_prefix(APPENDED),
        Some(text.as_ref()),
        "text_into"
    );
    let chars = text.chars().count();
    for max_chars in PREFIX_CHARS
        .into_iter()
        .chain([chars.saturating_sub(1), chars, chars + 1])
    {
        let prefix = part.text_prefix(max_chars);
        assert_eq!(
            prefix.as_deref(),
            Some(char_prefix(&text, max_chars)),
            "text_prefix({max_chars})"
        );
    }
    match part.kind() {
        PartKind::Html => {
            let html = part.html_utf8().expect("an HTML part has UTF-8 HTML");
            valid(&html);
            assert_eq!(html, strip_charset_meta(&text), "html_utf8");
        }
        _ => assert!(part.html_utf8().is_none()),
    }
}
