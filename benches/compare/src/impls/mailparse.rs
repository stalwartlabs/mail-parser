use crate::{tally::Tally, workload::Workload};
use ::mailparse::{
    MailAddr, MailHeaderMap, ParsedMail, addrparse_header, dateparse, msgidparse, parse_mail,
};
use std::hint::black_box;

const MAX_NESTING: usize = 8;

pub fn run(workload: Workload, raw: &[u8], tally: &mut Tally) -> bool {
    let Ok(mail) = parse_mail(black_box(raw)) else {
        return false;
    };
    if workload == Workload::Headers {
        headers(&mail, tally);
    }
    walk(&mail, workload, tally);
    true
}

fn media_type<'a>(mail: &'a ParsedMail<'_>) -> (&'a str, Option<&'a str>) {
    mail.ctype
        .mimetype
        .split_once('/')
        .map_or((mail.ctype.mimetype.as_str(), None), |(ctype, subtype)| {
            (ctype, Some(subtype))
        })
}

fn headers(mail: &ParsedMail<'_>, tally: &mut Tally) {
    let headers = &mail.headers;
    if let Some(subject) = headers.get_first_value("Subject") {
        tally.subject += 1;
        tally.text(&subject);
    }
    if let Some(from) = headers
        .get_first_header("From")
        .and_then(|header| addrparse_header(header).ok())
        && let Some(mailbox) = from.iter().find_map(|addr| match addr {
            MailAddr::Single(info) => Some(info),
            MailAddr::Group(group) => group.addrs.first(),
        })
    {
        tally.from += 1;
        tally.maybe_text(mailbox.display_name.as_deref());
        tally.text(&mailbox.addr);
    }
    if let Some(date) = headers
        .get_first_value("Date")
        .and_then(|date| dateparse(&date).ok())
    {
        tally.date += 1;
        tally.add(date as u64);
    }
    if let Some(ids) = headers
        .get_first_value("Message-ID")
        .and_then(|value| msgidparse(&value).ok())
        && let Some(id) = ids.first()
    {
        tally.message_id += 1;
        tally.text(id);
    }
}

fn walk(mail: &ParsedMail<'_>, workload: Workload, tally: &mut Tally) {
    tally.messages += 1;
    let mut nested = Vec::new();
    walk_tree(mail, workload, 0, tally, &mut nested);
    while let Some((bytes, depth)) = nested.pop() {
        match parse_mail(&bytes) {
            Ok(mail) => {
                tally.messages += 1;
                walk_tree(&mail, workload, depth, tally, &mut nested);
            }
            Err(_) => tally.failures += 1,
        }
    }
}

fn walk_tree(
    mail: &ParsedMail<'_>,
    workload: Workload,
    depth: usize,
    tally: &mut Tally,
    nested: &mut Vec<(Vec<u8>, usize)>,
) {
    let mut stack = vec![mail];
    while let Some(part) = stack.pop() {
        tally.parts += 1;
        let (ctype, subtype) = media_type(part);
        if workload == Workload::Structure {
            tally.content_type(ctype, subtype);
        }
        if !part.subparts.is_empty() || ctype == "multipart" {
            stack.extend(part.subparts.iter().rev());
        } else if ctype == "message"
            && matches!(subtype, Some("rfc822" | "global"))
            && depth < MAX_NESTING
        {
            match part.get_body_raw() {
                Ok(bytes) => nested.push((bytes, depth + 1)),
                Err(_) => tally.failures += 1,
            }
        } else if workload == Workload::Full {
            leaf(part, ctype, tally);
        }
    }
}

fn leaf(part: &ParsedMail<'_>, ctype: &str, tally: &mut Tally) {
    if ctype == "text" {
        match part.get_body() {
            Ok(text) => tally.leaf(text.as_bytes(), true),
            Err(_) => tally.failures += 1,
        }
    } else {
        match part.get_body_raw() {
            Ok(bytes) => tally.leaf(&bytes, false),
            Err(_) => tally.failures += 1,
        }
    }
}
