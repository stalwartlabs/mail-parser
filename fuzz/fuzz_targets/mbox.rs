#![no_main]

use libfuzzer_sys::fuzz_target;
use mail_parser_fuzz::mbox;

fuzz_target!(|data: &[u8]| {
    mbox::check(data);
});
