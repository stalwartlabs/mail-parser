use crate::{debug, display, valid};
use mail_parser::{
    Address, AddressList, ContentType, DateTime, HeaderValue, Host, Mailbox, Received, TextList,
};
use std::cmp::Ordering;

const ZONES: [i64; 6] = [0, 3_600, -39_600, 50_400, i64::MIN, i64::MAX];
const DAYS_PER_WEEK: u8 = 7;

pub fn header_value(value: HeaderValue<'_>) {
    assert_eq!(value.is_empty(), matches!(value, HeaderValue::Empty));
    match value {
        HeaderValue::Empty => assert_eq!(value.as_text(), None),
        HeaderValue::Text(text) => {
            valid(text);
            assert_eq!(value.as_text(), Some(text));
        }
        HeaderValue::TextList(list) => {
            text_list(list);
            assert_eq!(value.as_text(), list.last());
            assert_eq!(value.as_text_list(), Some(list));
        }
        HeaderValue::Address(list) => {
            addresses(list);
            assert_eq!(value.as_address(), Some(list));
        }
        HeaderValue::DateTime(date) => {
            datetime(date);
            assert_eq!(value.as_datetime(), Some(date));
        }
        HeaderValue::ContentType(ct) => {
            content_type(ct);
            assert_eq!(value.as_content_type(), Some(ct));
        }
        HeaderValue::Received(trace) => {
            received(trace);
            assert_eq!(value.as_received(), Some(trace));
        }
    }
    debug(&value);
}

pub fn text_list(list: TextList<'_>) {
    let len = list.len();
    assert_eq!(list.is_empty(), len == 0);
    assert_eq!(list.iter().len(), len);
    assert_eq!(list.iter().rev().count(), len);
    for (index, item) in list.iter().enumerate() {
        valid(item);
        assert_eq!(list.get(index), Some(item));
        assert!(list.contains(item));
    }
    assert_eq!(list.get(len), None);
    assert_eq!(list.first(), list.iter().next());
    assert_eq!(list.last(), list.iter().next_back());
    let copy = list;
    assert!(copy == list);
    debug(&list);
}

fn mailbox(mailbox: Mailbox<'_>, list: AddressList<'_>) {
    [mailbox.name(), mailbox.address()]
        .into_iter()
        .flatten()
        .for_each(valid);
    if let Some(address) = mailbox.address() {
        assert!(list.contains(address));
    }
}

pub fn addresses(list: AddressList<'_>) {
    let flattened = list.iter().flat_map(|address| {
        let (single, members) = match address {
            Address::Mailbox(mailbox) => (Some(mailbox), None),
            Address::Group(group) => (None, Some(group.mailboxes())),
        };
        single.into_iter().chain(members.into_iter().flatten())
    });
    assert!(
        flattened.eq(list.mailboxes()),
        "iter() and mailboxes() differ"
    );
    let grouped = list.groups().flat_map(|(_, run)| run);
    assert!(
        grouped.eq(list.mailboxes()),
        "groups() and mailboxes() differ"
    );
    for (name, run) in list.groups() {
        name.into_iter().for_each(valid);
        debug(&run);
    }
    let mut has_groups = false;
    for address in list.iter() {
        if let Address::Group(group) = address {
            has_groups = true;
            group.name().into_iter().for_each(valid);
            let members = group.mailboxes();
            assert_eq!(group.len(), members.len());
            assert_eq!(group.is_empty(), members.len() == 0);
            assert_eq!(members.rev().count(), group.len());
        }
        debug(&address);
    }
    assert_eq!(list.has_groups(), has_groups);
    assert_eq!(list.is_empty(), list.iter().next().is_none());
    assert_eq!(list.first(), list.mailboxes().next());
    assert_eq!(list.last(), list.mailboxes().last());
    for entry in list.mailboxes() {
        mailbox(entry, list);
    }
    let copy = list;
    assert!(copy == list);
    debug(&list);
}

pub fn content_type(ct: ContentType<'_>) {
    valid(ct.ctype());
    ct.subtype().into_iter().for_each(valid);
    let attributes = ct.attributes();
    let len = attributes.len();
    let mut count = 0;
    for (name, value) in attributes {
        valid(name);
        valid(value);
        assert!(ct.has_attribute(name));
        count += 1;
    }
    assert_eq!(count, len);
    assert_eq!(ct.attributes().rev().count(), len);
    let _ = (ct.is_attachment(), ct.is_inline());
    let copy = ct;
    assert!(copy == ct);
    debug(&ct);
}

fn host(host: Host<'_>) {
    if let Host::Name(name) = host {
        valid(name);
    }
    display(&host);
    debug(&host);
}

pub fn received(trace: Received<'_>) {
    [trace.from(), trace.by(), trace.helo()]
        .into_iter()
        .flatten()
        .for_each(host);
    [
        trace.from_iprev(),
        trace.for_(),
        trace.tls_cipher(),
        trace.id(),
        trace.ident(),
        trace.via(),
    ]
    .into_iter()
    .flatten()
    .for_each(valid);
    let _ = trace.from_ip();
    if let Some(protocol) = trace.with() {
        valid(protocol.as_str());
        display(&protocol);
    }
    if let Some(version) = trace.tls_version() {
        valid(version.as_str());
        display(&version);
    }
    if let Some(greeting) = trace.helo_cmd() {
        valid(greeting.as_str());
        display(&greeting);
    }
    trace.date().into_iter().for_each(datetime);
    let copy = trace;
    assert!(copy == trace);
    debug(&trace);
}

pub fn datetime(date: DateTime) {
    assert!(date.day_of_week() < DAYS_PER_WEEK);
    let _ = (date.julian_day(), date.to_timestamp_local());
    assert_eq!(date.cmp(&date), Ordering::Equal);
    valid(&date.to_rfc822());
    valid(&date.to_rfc3339());
    display(&date);
    for zone in ZONES {
        let shifted = date.to_timezone(zone);
        assert!(shifted.day_of_week() < DAYS_PER_WEEK);
    }
    let _ = i64::from(date);
    if date.is_valid() {
        let timestamp = date.to_timestamp();
        assert_eq!(
            DateTime::from_timestamp(timestamp).to_timestamp(),
            timestamp,
            "from_timestamp does not invert to_timestamp for {date:?}"
        );
        assert_eq!(
            DateTime::parse_rfc3339(&date.to_rfc3339()).map(|parsed| parsed.to_timestamp()),
            Some(timestamp),
            "RFC 3339 round trip of {date:?}"
        );
        assert_eq!(
            DateTime::parse_rfc822(&date.to_rfc822()).map(|parsed| parsed.to_timestamp()),
            Some(timestamp),
            "RFC 822 round trip of {date:?}"
        );
    }
}
