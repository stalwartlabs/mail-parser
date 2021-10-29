use mail_parser::Message;

#[test]
fn test_mail_parser() {
    let mut input = "pepe".to_string();
    let result = Message::parse(unsafe{input.as_bytes_mut()});

}
