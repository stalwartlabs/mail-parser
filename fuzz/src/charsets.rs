use crate::{char_prefix, debug, valid, walk};
use encodify::{base64, qp};
use mail_parser::{
    Charset, MessageParser, add_html_token,
    decoders::charsets::{self, charset_decoder},
    html_to_text, preview_html, preview_text, strip_charset_meta, text_to_html, truncate_html,
    truncate_text,
};
use std::borrow::Cow;

const LABELS: [&str; 44] = [
    "utf-8",
    "utf-7",
    "utf-16",
    "utf-16le",
    "utf-16be",
    "iso-8859-2",
    "iso-8859-3",
    "iso-8859-4",
    "iso-8859-5",
    "iso-8859-6",
    "iso-8859-7",
    "iso-8859-8",
    "iso-8859-9",
    "iso-8859-10",
    "iso-8859-13",
    "iso-8859-14",
    "iso-8859-15",
    "iso-8859-16",
    "windows-1250",
    "windows-1251",
    "windows-1252",
    "windows-1253",
    "windows-1254",
    "windows-1255",
    "windows-1256",
    "windows-1257",
    "windows-1258",
    "koi8-r",
    "koi8-u",
    "macintosh",
    "ibm850",
    "tis-620",
    "shift_jis",
    "big5",
    "euc-jp",
    "euc-kr",
    "gb18030",
    "gbk",
    "iso-2022-jp",
    "windows-874",
    "ibm866",
    "x-mac-cyrillic",
    "x-user-defined",
    "hz-gb-2312",
];

const PREVIEW_LENS: [usize; 6] = [0, 1, 6, 7, 20, 300];
const PREFIX_CHARS: [usize; 5] = [0, 1, 5, 64, 1_000];
const ELLIPSIS: &str = "...";
const APPENDED: &str = "appended";
const HTML_OPEN: &str = "<html><body>";
const HTML_CLOSE: &str = "</body></html>";
const MAX_LABEL: usize = 48;

#[derive(Debug, Clone, Copy)]
enum Transfer {
    Identity,
    Base64,
    QuotedPrintable,
}

const TRANSFERS: [(Transfer, &str); 3] = [
    (Transfer::Identity, "8bit"),
    (Transfer::Base64, "base64"),
    (Transfer::QuotedPrintable, "quoted-printable"),
];

pub fn check(data: &[u8]) {
    let mut pieces = data.splitn(2, |&byte| byte == b'\n');
    let (label, bytes) = match (pieces.next(), pieces.next()) {
        (Some(label), Some(rest)) => (label, rest),
        _ => (data.get(..MAX_LABEL).unwrap_or(data), data),
    };
    let charset = Charset::from_label(label);
    labels(label, charset, bytes);
    let fallback = data
        .first()
        .and_then(|&byte| LABELS.get(usize::from(byte) % LABELS.len()))
        .copied()
        .unwrap_or("utf-8");
    let label = match (charset, std::str::from_utf8(label)) {
        (Some(_), Ok(label)) if label.bytes().all(is_token) => label,
        _ => fallback,
    };
    let charset = Charset::from_label(label.as_bytes()).unwrap_or_default();
    let decoded = conversions(charset, bytes);
    for (transfer, name) in TRANSFERS {
        in_message(label, transfer, name, bytes, &decoded);
    }
    text_functions(&decoded);
    text_functions(&String::from_utf8_lossy(data));
}

fn is_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
}

fn labels(label: &[u8], charset: Option<Charset>, bytes: &[u8]) {
    assert_eq!(Charset::from_label(&label.to_ascii_uppercase()), charset);
    assert_eq!(Charset::from_label(&label.to_ascii_lowercase()), charset);
    let expected = charset.unwrap_or_default().decode(bytes);
    assert_eq!(charsets::decode(label, bytes), expected);
    let mut appended = String::from(APPENDED);
    charsets::decode_append(label, bytes, &mut appended);
    assert_eq!(appended.strip_prefix(APPENDED), Some(expected.as_ref()));
    match charset_decoder(label) {
        Some(decoder) => {
            assert!(charset.is_some_and(|charset| charset != Charset::Utf8));
            assert_eq!(decoder(bytes), expected);
        }
        None => assert!(charset.is_none_or(|charset| charset == Charset::Utf8)),
    }
    debug(&charset);
}

fn conversions(charset: Charset, bytes: &[u8]) -> Cow<'_, str> {
    let decoded = charset.decode(bytes);
    valid(&decoded);
    if let Cow::Borrowed(text) = &decoded {
        assert_eq!(
            text.as_bytes(),
            bytes,
            "borrowed text differs from its bytes"
        );
    }
    assert_eq!(
        charset.decode_owned(bytes.to_vec()),
        decoded,
        "decode_owned"
    );
    let mut appended = String::from(APPENDED);
    charset.decode_append(bytes, &mut appended);
    assert_eq!(
        appended.strip_prefix(APPENDED),
        Some(decoded.as_ref()),
        "decode_append"
    );
    decoded
}

fn in_message(label: &str, transfer: Transfer, name: &str, bytes: &[u8], decoded: &str) {
    let mut raw = format!(
        "Content-Type: text/plain; charset=\"{label}\"\r\nContent-Transfer-Encoding: {name}\r\n\r\n"
    )
    .into_bytes();
    match transfer {
        Transfer::Identity => raw.extend_from_slice(bytes),
        Transfer::Base64 => raw.extend_from_slice(base64::MIME.encode(bytes).as_bytes()),
        Transfer::QuotedPrintable => raw.extend_from_slice(qp::BINARY.encode(bytes).as_bytes()),
    }
    let message = MessageParser::new()
        .parse(&raw)
        .expect("a message with a header parses");
    walk::check_message(&message);
    let part = message.root().root_part();
    let body = part.decoded();
    if !matches!(transfer, Transfer::QuotedPrintable) {
        assert_eq!(body.as_ref(), bytes, "{transfer:?} body");
    }
    let text = part.text().expect("a text/plain part has text");
    if body.as_ref() == bytes {
        assert_eq!(text, decoded, "{label} text differs from Charset::decode");
    }
    let chars = text.chars().count();
    for max_chars in PREFIX_CHARS
        .into_iter()
        .chain([chars.saturating_sub(1), chars, chars + 1])
    {
        assert_eq!(
            part.text_prefix(max_chars).as_deref(),
            Some(char_prefix(&text, max_chars)),
            "{label} {transfer:?} text_prefix({max_chars})"
        );
    }
}

fn text_functions(text: &str) {
    let plain = html_to_text(text);
    valid(&plain);
    let html = text_to_html(text);
    valid(&html);
    assert!(html.starts_with(HTML_OPEN) && html.ends_with(HTML_CLOSE));
    assert!(!html.contains('\r'));
    valid(&strip_charset_meta(text));
    let mut token = String::new();
    add_html_token(&mut token, text.as_bytes(), true);
    valid(&token);
    let stripped = text.replace('\r', "");
    for max_len in PREVIEW_LENS {
        let preview = preview_text(text, max_len);
        check_cut(&preview, &stripped, max_len);
        assert_eq!(truncate_text(text, max_len), preview);
        let html_preview = preview_html(text, max_len);
        valid(&html_preview);
        assert!(html_preview.len() <= max_len && !html_preview.contains('\r'));
        assert_eq!(
            html_preview,
            preview_text(&plain, max_len),
            "preview_html differs from preview_text of html_to_text"
        );
        let truncated = truncate_html(text, max_len);
        valid(&truncated);
        assert!(truncated.len() <= max_len && !truncated.contains('\r'));
        if stripped.len() <= max_len {
            assert_eq!(truncated, stripped);
        }
    }
}

fn check_cut(preview: &str, stripped: &str, max_len: usize) {
    valid(preview);
    assert!(preview.len() <= max_len, "preview longer than {max_len}");
    assert!(!preview.contains('\r'));
    if stripped.len() <= max_len {
        assert_eq!(preview, stripped);
        return;
    }
    let kept = if stripped.starts_with(preview) {
        preview
    } else {
        preview
            .strip_suffix(ELLIPSIS)
            .expect("a cut preview ends with an ellipsis")
    };
    assert!(stripped.starts_with(kept), "preview is not a prefix");
}
