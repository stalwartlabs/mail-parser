#![no_main]

use libfuzzer_sys::fuzz_target;
use mail_parser_fuzz::fields;

fuzz_target!(|data: &[u8]| {
    fields::check(data);
});
