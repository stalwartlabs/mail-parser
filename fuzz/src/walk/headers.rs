use super::values::header_value;
use crate::{debug, valid};
use mail_parser::{Header, HeaderForm, HeaderName, HeaderValue, Headers};

pub const FORMS: [HeaderForm; 9] = [
    HeaderForm::Raw,
    HeaderForm::Text,
    HeaderForm::Addresses,
    HeaderForm::MessageIds,
    HeaderForm::CommaList,
    HeaderForm::Date,
    HeaderForm::ContentType,
    HeaderForm::Received,
    HeaderForm::Ignore,
];

const CHECKED_LOOKUPS: usize = 64;
const CHECKED_REPARSES: usize = 256;
const MAX_LINE_BREAK: usize = 2;

macro_rules! getters {
    ($target:expr) => {{
        let target = &$target;
        for text in [
            target.subject(),
            target.comments(),
            target.mime_version(),
            target.thread_name(),
            target.message_id(),
            target.return_address(),
        ]
        .into_iter()
        .flatten()
        {
            $crate::valid(text);
        }
        assert_eq!(
            target.thread_name(),
            target.subject().map(mail_parser::thread_name)
        );
        for list in [
            target.from(),
            target.to(),
            target.cc(),
            target.bcc(),
            target.reply_to(),
            target.sender(),
            target.resent_to(),
            target.resent_from(),
            target.resent_cc(),
            target.resent_bcc(),
            target.resent_sender(),
            target.list_archive(),
            target.list_help(),
            target.list_id(),
            target.list_owner(),
            target.list_post(),
            target.list_subscribe(),
            target.list_unsubscribe(),
        ]
        .into_iter()
        .flatten()
        .chain(target.all_to())
        .chain(target.all_cc())
        .chain(target.all_bcc())
        {
            $crate::walk::addresses(list);
        }
        for date in [target.date(), target.resent_date()].into_iter().flatten() {
            $crate::walk::datetime(date);
        }
        for list in [
            target.in_reply_to(),
            target.references(),
            target.resent_message_id(),
            target.return_path(),
            target.keywords(),
        ]
        .into_iter()
        .flatten()
        {
            $crate::walk::text_list(list);
        }
        for trace in target.received().into_iter().chain(target.all_received()) {
            $crate::walk::received(trace);
        }
    }};
}

pub(crate) use getters;

pub fn check_headers(headers: Headers<'_>, source: &[u8], part_header: usize, body: usize) {
    let len = headers.len();
    assert_eq!(headers.is_empty(), len == 0);
    assert_eq!(headers.iter().len(), len);
    assert_eq!(headers.iter().rev().count(), len);
    assert_eq!((&headers).into_iter().count(), len);
    let mut previous = None;
    for (index, header) in headers.iter().enumerate() {
        check_offsets(header, source, part_header, body);
        assert!(
            previous < Some(header.offset_field()),
            "headers out of order"
        );
        previous = Some(header.offset_field());
        check_name(headers, header, index < CHECKED_LOOKUPS);
        if index < CHECKED_REPARSES {
            check_value(header);
        } else {
            header_value(header.value());
        }
        debug(&header);
    }
    let _ = headers.has_known();
    getters!(headers);
    debug(&headers);
}

fn check_offsets(header: Header<'_>, source: &[u8], part_header: usize, body: usize) {
    let (field, start, end) = (
        header.offset_field() as usize,
        header.offset_start() as usize,
        header.offset_end() as usize,
    );
    assert!(
        part_header <= field && field < start && start <= end && end <= source.len(),
        "header {:?} offsets {field}..{start}..{end}, part header {part_header}, source {}",
        header.raw_name(),
        source.len()
    );
    assert!(start <= body, "header value starts past the body");
    if end > body {
        let tail = source.get(body..end).unwrap_or_default();
        assert!(
            tail.len() <= MAX_LINE_BREAK && tail.iter().all(|byte| matches!(byte, b'\r' | b'\n')),
            "header {:?} ends {} bytes past the body offset",
            header.raw_name(),
            end - body
        );
    }
    assert_eq!(source.get(start..end), Some(header.raw_value()));
    let name = header.raw_name();
    valid(name);
    let region = source.get(field..start).unwrap_or_default();
    let found = if name.contains(char::REPLACEMENT_CHARACTER) {
        String::from_utf8_lossy(region).contains(name)
    } else {
        region
            .windows(name.len())
            .any(|window| window == name.as_bytes())
    };
    assert!(
        name.is_empty() || found,
        "raw_name {name:?} not in its region {region:?}"
    );
}

fn check_name(headers: Headers<'_>, header: Header<'_>, lookups: bool) {
    let name = header.name();
    let raw = header.raw_name();
    valid(name.as_str());
    match &name {
        HeaderName::Other(other) => assert_eq!(other.as_ref(), raw),
        known => assert!(
            known.as_str().eq_ignore_ascii_case(raw),
            "known name {known:?} spelled {raw:?}"
        ),
    }
    if !lookups {
        return;
    }
    let last = headers
        .get(&name)
        .expect("get finds the name of a present header");
    assert!(last.offset_field() >= header.offset_field());
    assert!(
        headers
            .all(&name)
            .any(|same| same.offset_field() == header.offset_field())
    );
    assert!(headers.contains(&name));
    assert_eq!(headers.value(&name), Some(last.value()));
    if !raw.is_empty() {
        let by_raw = headers.get(raw).map(|found| found.offset_field());
        assert_eq!(by_raw, Some(last.offset_field()));
    }
}

fn own_forms(value: HeaderValue<'_>) -> &'static [HeaderForm] {
    match value {
        HeaderValue::Empty => &[],
        HeaderValue::Text(_) => &[HeaderForm::Text, HeaderForm::Raw],
        HeaderValue::TextList(_) => &[HeaderForm::MessageIds, HeaderForm::CommaList],
        HeaderValue::Address(_) => &[HeaderForm::Addresses],
        HeaderValue::DateTime(_) => &[HeaderForm::Date],
        HeaderValue::ContentType(_) => &[HeaderForm::ContentType],
        HeaderValue::Received(_) => &[HeaderForm::Received],
    }
}

fn check_value(header: Header<'_>) {
    let value = header.value();
    header_value(value);
    let forms = own_forms(value);
    let mut matched = forms.is_empty();
    for form in FORMS {
        let parsed = header.parse_as(form);
        let reparsed = parsed.value();
        header_value(reparsed);
        matched |= forms.contains(&form) && reparsed == value;
        assert!(
            form.parse(header.raw_value()) == parsed,
            "parse_as({form:?}) differs from HeaderForm::parse of the raw value"
        );
        debug(&parsed);
    }
    assert!(
        matched,
        "value of {:?} differs from parse_as in its own form",
        header.raw_name()
    );
}
