use crate::{debug, display, valid, walk, within};
use mail_parser::{
    DateTime, HeaderForm, HeaderName, MessageParser, parse_address_detail_part,
    parse_address_domain, parse_address_local_part, parse_address_user_part, thread_name,
};
use std::{
    borrow::Cow,
    cmp::Ordering,
    hash::{DefaultHasher, Hash, Hasher},
    sync::LazyLock,
};

const FIELD: &str = "X-Fuzz";
const FIELD_END: &[u8] = b"\n\n";

static PARSERS: LazyLock<[MessageParser; 9]> =
    LazyLock::new(|| walk::FORMS.map(|form| MessageParser::new().header(FIELD, form)));

type AddressHelper = fn(&str) -> Option<&str>;

const ADDRESS_HELPERS: [AddressHelper; 4] = [
    parse_address_local_part,
    parse_address_domain,
    parse_address_user_part,
    parse_address_detail_part,
];

pub fn check(data: &[u8]) {
    for form in walk::FORMS {
        let parsed = form.parse(data);
        walk::header_value(parsed.value());
        debug(&parsed);
    }
    in_message(data);
    let text = String::from_utf8_lossy(data);
    dates(&text, data);
    names(&text);
    if let Some((name, _)) = text.split_once(':') {
        names(name);
        names(name.trim());
    }
    for helper in ADDRESS_HELPERS {
        if let Some(part) = helper(&text) {
            valid(part);
            assert!(within(part.as_bytes(), text.as_bytes()));
        }
    }
    let thread = thread_name(&text);
    valid(thread);
    assert!(within(thread.as_bytes(), text.as_bytes()));
}

fn in_message(data: &[u8]) {
    let mut raw = Vec::with_capacity(FIELD.len() + 1 + data.len() + FIELD_END.len());
    raw.extend_from_slice(FIELD.as_bytes());
    raw.push(b':');
    raw.extend_from_slice(data);
    raw.extend_from_slice(FIELD_END);
    for (parser, form) in PARSERS.iter().zip(walk::FORMS) {
        let message = parser.parse(&raw).expect("a message with a header parses");
        let header = message
            .headers()
            .iter()
            .next()
            .expect("the message has its header");
        assert_eq!(header.raw_name(), FIELD);
        let parsed = header.parse_as(form);
        assert_eq!(
            parsed.value(),
            header.value(),
            "{form:?}: in-message value differs from parse_as"
        );
        assert!(
            form.parse(header.raw_value()) == parsed,
            "{form:?}: HeaderForm::parse differs from parse_as"
        );
    }
}

fn dates(text: &str, data: &[u8]) {
    let rfc822 = DateTime::parse_rfc822(text);
    let header = HeaderForm::Date.parse(text.as_bytes());
    assert_eq!(
        rfc822,
        header.value().as_datetime(),
        "parse_rfc822 differs from the Date form"
    );
    for date in [rfc822, DateTime::parse_rfc3339(text)]
        .into_iter()
        .flatten()
    {
        walk::datetime(date);
    }
    if let Some(bytes) = data.first_chunk::<8>() {
        walk::datetime(DateTime::from_timestamp(i64::from_le_bytes(*bytes)));
    }
}

fn is_ftext(byte: u8) -> bool {
    matches!(byte, b'!'..=b'9' | b';'..=b'~')
}

fn fingerprint(name: &HeaderName<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

fn same_name(left: &HeaderName<'_>, right: &HeaderName<'_>) {
    assert_eq!(left, right);
    assert_eq!(left.cmp(right), Ordering::Equal);
    assert_eq!(fingerprint(left), fingerprint(right));
}

fn names(candidate: &str) {
    let parsed = HeaderName::parse(candidate);
    let allowed = !candidate.is_empty() && candidate.bytes().all(is_ftext);
    assert_eq!(
        parsed.is_some(),
        allowed,
        "HeaderName::parse({candidate:?})"
    );
    let converted = HeaderName::from(candidate);
    check_name(&converted, candidate);
    if let Some(name) = &parsed {
        same_name(name, &converted);
        check_name(name, candidate);
    }
    same_name(&converted, &HeaderName::from(candidate.to_string()));
    same_name(&converted, &HeaderName::from(Cow::Borrowed(candidate)));
    same_name(&converted, &HeaderName::Other(Cow::Borrowed(candidate)));
    same_name(
        &converted,
        &HeaderName::Other(Cow::Owned(candidate.to_ascii_uppercase())),
    );
}

fn check_name(name: &HeaderName<'_>, spelling: &str) {
    let text = name.as_str();
    valid(text);
    assert!(
        text.eq_ignore_ascii_case(spelling),
        "{name:?} spelled {spelling:?}"
    );
    display(name);
    debug(name);
    same_name(&name.clone().into_owned(), name);
    assert_eq!(Cow::from(name.clone()), text);
    assert_eq!(String::from(name.clone()), text);
    let _ = (
        name.is_other(),
        name.is_mime_header(),
        name.is_structured(),
        name.as_static_str(),
    );
    let json = serde_json::to_string(name).expect("a header name serializes");
    let back: HeaderName<'static> =
        serde_json::from_str(&json).expect("a serialized header name deserializes");
    same_name(&back, name);
}
