use std::borrow::{BorrowMut, Cow};

use crate::decoders::buffer_writer::BufferWriter;

use self::{
    address::{parse_address, Address},
    content_type::ContentType,
    date::DateTime,
    id::parse_id,
    list::parse_comma_separared,
    raw::{parse_and_ignore, parse_raw},
    unstructured::parse_unstructured,
};

use super::{
    header::{MessageHeader, MimeHeader},
    message_stream::MessageStream,
};

pub mod address;
pub mod content_type;
pub mod date;
pub mod id;
pub mod list;
pub mod raw;
pub mod unstructured;

pub trait MessageField<'x> {
    fn set_date(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_sender(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_received(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_references(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_cc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_comments(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_cc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_content_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_message_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_reply_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_bcc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_subject(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_keywords(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_help(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_owner(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_date(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_bcc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_from(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_content_transfer_encoding(
        &mut self,
        stream: &MessageStream<'x>,
        buffer: &BufferWriter<'x>,
    );
    fn set_return_path(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_archive(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_sender(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_subscribe(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_message_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_content_type(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_post(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_in_reply_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_content_description(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_resent_from(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_content_disposition(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_list_unsubscribe(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_mime_version(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>);
    fn set_unsupported(
        &mut self,
        stream: &MessageStream<'x>,
        buffer: &BufferWriter<'x>,
        name: &'x [u8],
    );
    fn get_content_description(&self) -> Option<&Cow<'x, str>>;
    fn get_content_disposition(&self) -> Option<&ContentType<'x>>;
    fn get_content_id(&self) -> Option<&Cow<'x, str>>;
    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>>;
    fn get_content_type(&self) -> Option<&ContentType<'x>>;
}

impl<'x> MessageField<'x> for MessageHeader<'x> {
    fn set_date(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.date = self::date::parse_date(stream, false);
    }

    fn set_sender(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.sender = parse_address(stream, buffer);
    }

    fn set_received(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_line(&mut self.received, parse_raw(stream));
    }

    fn set_references(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.references = parse_id(stream);
    }

    fn set_cc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.cc = parse_address(stream, buffer);
    }

    fn set_comments(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_line(&mut self.comments, parse_unstructured(stream, buffer));
    }

    fn set_resent_cc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_address_line(&mut self.resent_cc, parse_address(stream, buffer));
    }

    fn set_content_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.mime.set_content_id(stream, buffer);
    }

    fn set_resent_message_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_line_many(&mut self.resent_message_id, parse_id(stream));
    }

    fn set_reply_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.reply_to = parse_address(stream, buffer);
    }

    fn set_resent_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_address_line(&mut self.resent_to, parse_address(stream, buffer));
    }

    fn set_resent_bcc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_address_line(&mut self.resent_bcc, parse_address(stream, buffer));
    }

    fn set_subject(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.subject = parse_unstructured(stream, buffer);
    }

    fn set_keywords(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_line_many(&mut self.keywords, parse_comma_separared(stream, buffer));
    }

    fn set_list_help(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_help = parse_address(stream, buffer);
    }

    fn set_list_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_id = parse_address(stream, buffer);
    }

    fn set_list_owner(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_owner = parse_address(stream, buffer);
    }

    fn set_resent_date(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_date_line(&mut self.resent_date, self::date::parse_date(stream, false));
    }

    fn set_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.to = parse_address(stream, buffer);
    }

    fn set_bcc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.bcc = parse_address(stream, buffer);
    }

    fn set_from(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.from = parse_address(stream, buffer);
    }

    fn set_content_transfer_encoding(
        &mut self,
        stream: &MessageStream<'x>,
        buffer: &BufferWriter<'x>,
    ) {
        self.mime.set_content_transfer_encoding(stream, buffer);
    }

    fn set_return_path(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_line_many(&mut self.return_path, parse_id(stream));
    }

    fn set_list_archive(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_archive = parse_address(stream, buffer);
    }

    fn set_resent_sender(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_address_line(&mut self.resent_sender, parse_address(stream, buffer));
    }

    fn set_list_subscribe(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_subscribe = parse_address(stream, buffer);
    }

    fn set_message_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.message_id = parse_id(stream).map(|mut v| v.pop().unwrap());
    }

    fn set_content_type(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.mime.set_content_type(stream, buffer);
    }

    fn set_list_post(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_post = parse_address(stream, buffer);
    }

    fn set_in_reply_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.reply_to = parse_address(stream, buffer);
    }

    fn set_content_description(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.mime.set_content_description(stream, buffer);
    }

    fn set_resent_from(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        append_address_line(&mut self.resent_from, parse_address(stream, buffer));
    }

    fn set_content_disposition(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.mime.set_content_disposition(stream, buffer);
    }

    fn set_list_unsubscribe(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.list_unsubscribe = parse_address(stream, buffer);
    }

    fn set_mime_version(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.mime_version = parse_raw(stream);
    }

    fn set_unsupported(
        &mut self,
        stream: &MessageStream<'x>,
        buffer: &BufferWriter<'x>,
        name: &'x [u8],
    ) {
        if let Ok(name) = std::str::from_utf8(name) {
            if let Some(value) = parse_unstructured(stream, buffer) {
                if let Some(arr) = self.others.get_mut(name) {
                    arr.push(value);
                } else {
                    self.others.insert(name, vec![value]);
                }
            }
        }
    }

    fn get_content_description(&self) -> Option<&Cow<'x, str>> {
        self.mime.content_description.as_ref()
    }

    fn get_content_disposition(&self) -> Option<&ContentType<'x>> {
        self.mime.content_disposition.as_ref()
    }

    fn get_content_id(&self) -> Option<&Cow<'x, str>> {
        self.mime.content_id.as_ref()
    }

    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>> {
        self.mime.content_transfer_encoding.as_ref()
    }

    fn get_content_type(&self) -> Option<&ContentType<'x>> {
        self.mime.content_type.as_ref()
    }
}

impl<'x> MessageField<'x> for MimeHeader<'x> {
    fn set_date(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_sender(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_received(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_references(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_cc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_comments(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_resent_cc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_content_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.content_id = parse_id(stream).map(|mut v| v.pop().unwrap());
    }

    fn set_resent_message_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_reply_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_resent_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_resent_bcc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_subject(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_keywords(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_list_help(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_list_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_list_owner(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_resent_date(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_bcc(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_from(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_content_transfer_encoding(
        &mut self,
        stream: &MessageStream<'x>,
        buffer: &BufferWriter<'x>,
    ) {
        self.content_transfer_encoding = parse_unstructured(stream, buffer);
    }

    fn set_return_path(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_list_archive(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_resent_sender(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_list_subscribe(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_message_id(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_content_type(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.content_type = self::content_type::parse_content_type(stream, buffer);
    }

    fn set_list_post(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_in_reply_to(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_content_description(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.content_description = parse_unstructured(stream, buffer);
    }

    fn set_resent_from(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_list_unsubscribe(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_mime_version(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        parse_and_ignore(stream);
    }

    fn set_unsupported(
        &mut self,
        stream: &MessageStream<'x>,
        buffer: &BufferWriter<'x>,
        _name: &'x [u8],
    ) {
        parse_and_ignore(stream);
    }

    fn set_content_disposition(&mut self, stream: &MessageStream<'x>, buffer: &BufferWriter<'x>) {
        self.content_disposition = self::content_type::parse_content_type(stream, buffer);
    }

    fn get_content_description(&self) -> Option<&Cow<'x, str>> {
        self.content_description.as_ref()
    }

    fn get_content_disposition(&self) -> Option<&ContentType<'x>> {
        self.content_disposition.as_ref()
    }

    fn get_content_id(&self) -> Option<&Cow<'x, str>> {
        self.content_id.as_ref()
    }

    fn get_content_transfer_encoding(&self) -> Option<&Cow<'x, str>> {
        self.content_transfer_encoding.as_ref()
    }

    fn get_content_type(&self) -> Option<&ContentType<'x>> {
        self.content_type.as_ref()
    }
}

pub fn parse_date<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_date(stream, buffer);
}

pub fn parse_sender<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_sender(stream, buffer);
}

pub fn parse_received<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_received(stream, buffer);
}

pub fn parse_references<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_references(stream, buffer);
}

pub fn parse_cc<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_cc(stream, buffer);
}

pub fn parse_comments<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_comments(stream, buffer);
}

pub fn parse_resent_cc<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_cc(stream, buffer);
}

pub fn parse_content_id<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_content_id(stream, buffer);
}

pub fn parse_resent_message_id<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_message_id(stream, buffer);
}

pub fn parse_reply_to<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_reply_to(stream, buffer);
}

pub fn parse_resent_to<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_to(stream, buffer);
}

pub fn parse_resent_bcc<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_bcc(stream, buffer);
}

pub fn parse_subject<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_subject(stream, buffer);
}

pub fn parse_keywords<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_keywords(stream, buffer);
}

pub fn parse_list_help<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_help(stream, buffer);
}

pub fn parse_list_id<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_id(stream, buffer);
}

pub fn parse_list_owner<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_owner(stream, buffer);
}

pub fn parse_resent_date<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_date(stream, buffer);
}

pub fn parse_to<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_to(stream, buffer);
}

pub fn parse_bcc<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_bcc(stream, buffer);
}

pub fn parse_from<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_from(stream, buffer);
}

pub fn parse_content_transfer_encoding<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_content_transfer_encoding(stream, buffer);
}

pub fn parse_return_path<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_return_path(stream, buffer);
}

pub fn parse_list_archive<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_archive(stream, buffer);
}

pub fn parse_resent_sender<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_sender(stream, buffer);
}

pub fn parse_list_subscribe<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_subscribe(stream, buffer);
}

pub fn parse_message_id<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_message_id(stream, buffer);
}

pub fn parse_content_type<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_content_type(stream, buffer);
}

pub fn parse_list_post<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_post(stream, buffer);
}

pub fn parse_in_reply_to<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_in_reply_to(stream, buffer);
}

pub fn parse_content_description<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_content_description(stream, buffer);
}

pub fn parse_resent_from<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_resent_from(stream, buffer);
}

pub fn parse_content_disposition<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_content_disposition(stream, buffer);
}

pub fn parse_list_unsubscribe<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_list_unsubscribe(stream, buffer);
}

pub fn parse_mime_version<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
    header.set_mime_version(stream, buffer);
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

pub fn parse_no_op<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
) {
}

pub fn parse_unsupported<'x>(
    header: &mut dyn MessageField<'x>,
    stream: &MessageStream<'x>,
    buffer: &BufferWriter<'x>,
    name: &'x [u8],
) {
    header.set_unsupported(stream, buffer, name);
}
