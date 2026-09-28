#![no_main]

use libfuzzer_sys::fuzz_target;
use mail_parser_fuzz::kernels;
use std::sync::Once;

static REPORTED: Once = Once::new();

fuzz_target!(|data: &[u8]| {
    REPORTED.call_once(kernels::report);
    kernels::check(data);
});
