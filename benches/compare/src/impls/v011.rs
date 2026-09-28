use crate::{tally::Tally, workload::Workload};
use mail_parser_011::{Message, MessageParser, MimeHeaders, PartType};
use std::hint::black_box;

pub fn run(parser: &MessageParser, workload: Workload, raw: &[u8], tally: &mut Tally) -> bool {
    let Some(message) = parser.parse(black_box(raw)) else {
        return false;
    };
    match workload {
        Workload::Structure => for_each_message(&message, tally, structure),
        Workload::Headers => headers(&message, tally),
        Workload::Full => for_each_message(&message, tally, full),
    }
    true
}

fn for_each_message(message: &Message<'_>, tally: &mut Tally, visit: fn(&Message<'_>, &mut Tally)) {
    tally.messages += 1;
    visit(message, tally);
    for part in &message.parts {
        if let PartType::Message(nested) = &part.body {
            for_each_message(nested, tally, visit);
        }
    }
}

fn structure(message: &Message<'_>, tally: &mut Tally) {
    for part in &message.parts {
        tally.parts += 1;
        if let Some(content_type) = part.content_type() {
            tally.content_type(content_type.ctype(), content_type.subtype());
        }
    }
}

fn headers(message: &Message<'_>, tally: &mut Tally) {
    tally.messages += 1;
    if let Some(subject) = message.subject() {
        tally.subject += 1;
        tally.text(subject);
    }
    if let Some(addr) = message.from().and_then(|from| from.first()) {
        tally.from += 1;
        tally.maybe_text(addr.name.as_deref());
        tally.maybe_text(addr.address.as_deref());
    }
    if let Some(date) = message.date() {
        tally.date += 1;
        tally.add(date.to_timestamp() as u64);
    }
    if let Some(id) = message.message_id() {
        tally.message_id += 1;
        tally.text(id);
    }
}

fn full(message: &Message<'_>, tally: &mut Tally) {
    for part in &message.parts {
        tally.parts += 1;
        match &part.body {
            PartType::Text(text) | PartType::Html(text) => tally.leaf(text.as_bytes(), true),
            PartType::Binary(bytes) | PartType::InlineBinary(bytes) => tally.leaf(bytes, false),
            PartType::Message(_) | PartType::Multipart(_) => {}
        }
    }
}
