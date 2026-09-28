#![no_main]

use libfuzzer_sys::fuzz_target;
use mail_parser::MessageParser;
use mail_parser_fuzz::walk;
use std::sync::LazyLock;

static PARSER: LazyLock<MessageParser> = LazyLock::new(MessageParser::new);

fuzz_target!(|data: &[u8]| {
    walk::run(&PARSER, data, None);
});
