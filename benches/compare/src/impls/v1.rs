use crate::{tally::Tally, workload::Workload};
use mail_parser::{Message, MessageParser, PartKind};
use std::hint::black_box;

pub fn run(parser: &MessageParser, workload: Workload, raw: &[u8], tally: &mut Tally) -> bool {
    let Some(message) = parser.parse(black_box(raw)) else {
        return false;
    };
    match workload {
        Workload::Structure => structure(&message, tally),
        Workload::Headers => headers(&message, tally),
        Workload::Full => full(&message, tally),
    }
    true
}

fn structure(message: &Message<'_>, tally: &mut Tally) {
    tally.messages += message.messages().len() as u64;
    for part in message.parts() {
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
    if let Some(mailbox) = message.from().and_then(|from| from.first()) {
        tally.from += 1;
        tally.maybe_text(mailbox.name());
        tally.maybe_text(mailbox.address());
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
    tally.messages += message.messages().len() as u64;
    for part in message.parts() {
        tally.parts += 1;
        match part.kind() {
            PartKind::Text | PartKind::Html => match part.text() {
                Some(text) => tally.leaf(text.as_bytes(), true),
                None => tally.failures += 1,
            },
            PartKind::Binary | PartKind::InlineBinary => tally.leaf(&part.decoded(), false),
            PartKind::Multipart | PartKind::Message(_) => {}
        }
    }
}
