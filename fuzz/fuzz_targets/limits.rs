#![no_main]

use libfuzzer_sys::fuzz_target;
use mail_parser::MessageParser;
use mail_parser_fuzz::walk::{self, Limits};

const MAX_DEPTH: u8 = 8;
const MAX_PARTS: u8 = 24;
const MAX_ENCODED: u8 = 4;

fuzz_target!(|data: &[u8]| {
    let Some(([depth, parts, encoded], raw)) = data.split_first_chunk::<3>() else {
        return;
    };
    let limits = Limits {
        depth: usize::from(depth % MAX_DEPTH) + 1,
        parts: usize::from(parts % MAX_PARTS) + 1,
        encoded: usize::from(encoded % MAX_ENCODED),
    };
    let parser = MessageParser::new()
        .max_depth(limits.depth)
        .max_parts(limits.parts)
        .max_encoded_nesting(limits.encoded);
    walk::run(&parser, raw, Some(limits));
});
