use std::{borrow::Cow, collections::hash_map::Entry};

use self::{
    address::{parse_address, Address},
    date::DateTime,
    id::parse_id,
    list::parse_comma_separared,
    raw::parse_raw,
    unstructured::parse_unstructured,
};

use super::{header::Header, message_stream::MessageStream};

pub mod address;
pub mod content_type;
pub mod date;
pub mod id;
pub mod list;
pub mod raw;
pub mod unstructured;

pub fn parse_date<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.date = self::date::parse_date(stream, false);
}

pub fn parse_sender<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.sender = parse_address(stream);
}

pub fn parse_received<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_line(&mut header.received, parse_raw(stream));
}

pub fn parse_references<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.references = parse_id(stream);
}

pub fn parse_cc<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.cc = parse_address(stream);
}

pub fn parse_comments<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_line(&mut header.comments, parse_unstructured(stream));
}

pub fn parse_resent_cc<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_address_line(&mut header.resent_cc, parse_address(stream));
}

pub fn parse_content_id<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.content_id = parse_id(stream).map(|mut v| v.pop().unwrap());
}

pub fn parse_resent_message_id<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_line_many(&mut header.resent_message_id, parse_id(stream));
}

pub fn parse_reply_to<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.reply_to = parse_address(stream);
}

pub fn parse_resent_to<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_address_line(&mut header.resent_to, parse_address(stream));
}

pub fn parse_resent_bcc<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_address_line(&mut header.resent_bcc, parse_address(stream));
}

pub fn parse_subject<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.subject = parse_unstructured(stream);
}

pub fn parse_keywords<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_line_many(&mut header.keywords, parse_comma_separared(stream));
}

pub fn parse_list_help<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_help = parse_address(stream);
}

pub fn parse_list_id<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_id = parse_address(stream);
}

pub fn parse_list_owner<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_owner = parse_address(stream);
}

pub fn parse_resent_date<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_date_line(
        &mut header.resent_date,
        self::date::parse_date(stream, false),
    );
}

pub fn parse_to<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.to = parse_address(stream);
}

pub fn parse_bcc<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.bcc = parse_address(stream);
}

pub fn parse_from<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.from = parse_address(stream);
}

pub fn parse_content_transfer_encoding<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.content_transfer_encoding = parse_unstructured(stream);
}

pub fn parse_return_path<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_line(&mut header.return_path, parse_raw(stream));
}

pub fn parse_list_archive<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_archive = parse_address(stream);
}

pub fn parse_resent_sender<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_address_line(&mut header.resent_sender, parse_address(stream));
}

pub fn parse_list_subscribe<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_subscribe = parse_address(stream);
}

pub fn parse_message_id<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.message_id = parse_id(stream).map(|mut v| v.pop().unwrap());
}

pub fn parse_content_type<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.content_type = self::content_type::parse_content_type(stream);
}

pub fn parse_list_post<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_post = parse_address(stream);
}

pub fn parse_in_reply_to<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.reply_to = parse_address(stream);
}

pub fn parse_content_description<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.content_description = parse_unstructured(stream);
}

pub fn parse_resent_from<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    append_address_line(&mut header.resent_from, parse_address(stream));
}

pub fn parse_content_disposition<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.content_disposition = self::content_type::parse_content_type(stream);
}

pub fn parse_list_unsubscribe<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.list_unsubscribe = parse_address(stream);
}

pub fn parse_mime_version<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {
    header.mime_version = parse_raw(stream);
}

fn append_address_line<'x>(value: &mut Address<'x>, new_value: Address<'x>) {
    if new_value != Address::Empty {
        match value {
            Address::Collection(ref mut arr) => {
                arr.push(new_value);
            }
            Address::Empty => *value = new_value,
            _ => {
                *value = Address::Collection(vec![std::mem::take(value), new_value]);
            }
        }
    }
}

fn append_date_line<'x>(value: &mut Option<Vec<DateTime>>, new_value: Option<DateTime>) {
    if let Some(new_value) = new_value {
        if let Some(value) = value {
            value.push(new_value);
        } else {
            *value = Some(vec![new_value]);
        }
    }
}

fn append_line<'x>(value: &mut Option<Vec<Cow<'x, str>>>, new_value: Option<Cow<'x, str>>) {
    if let Some(new_value) = new_value {
        if let Some(value) = value {
            value.push(new_value);
        } else {
            *value = Some(vec![new_value]);
        }
    }
}

fn append_line_many<'x>(
    value: &mut Option<Vec<Cow<'x, str>>>,
    new_value: Option<Vec<Cow<'x, str>>>,
) {
    if let Some(mut new_value) = new_value {
        if let Some(value) = value {
            value.append(&mut new_value);
        } else {
            *value = Some(new_value);
        }
    }
}

pub fn parse_no_op<'x>(header: &mut Header<'x>, stream: &'x MessageStream<'x>) {}

pub fn parse_unsupported<'x>(header: &mut Header<'x>, stream: &'x MessageStream, name: &'x [u8]) {
    if let Ok(name) = std::str::from_utf8(name) {
        if let Some(value) = parse_unstructured(stream) {
            if let Some(arr) = header.others.get_mut(name) {
                arr.push(value);
            } else {
                header.others.insert(name, vec![value]);
            }
        }
    }
}
