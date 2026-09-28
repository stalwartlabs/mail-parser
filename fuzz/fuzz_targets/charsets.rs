#![no_main]

use libfuzzer_sys::fuzz_target;
use mail_parser_fuzz::charsets;

fuzz_target!(|data: &[u8]| {
    charsets::check(data);
});
