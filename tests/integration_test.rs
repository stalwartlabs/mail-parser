use mail_parser::{Message, MimeFieldGet};

#[test]
fn test_mail_parser() {
    let mut input = "test".to_string();
    let message = Message::parse(unsafe{input.as_bytes_mut()});

    message.get_content_id();
    message.get_subject();

}
