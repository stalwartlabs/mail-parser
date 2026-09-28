#![allow(unsafe_code)]

use crate::{tally::Tally, workload::Workload};
use std::{hint::black_box, sync::Once};

type Entry = unsafe extern "C" fn(*const u8, usize, *mut Tally) -> i32;

unsafe extern "C" {
    fn cmp_gmime_init();
    fn cmp_gmime_structure(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_gmime_headers(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_gmime_full(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_vmime_structure(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_vmime_headers(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_vmime_full(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_libetpan_structure(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_libetpan_headers(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_libetpan_full(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_dovecot_init();
    fn cmp_dovecot_structure(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_dovecot_headers(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
    fn cmp_dovecot_full(raw: *const u8, len: usize, tally: *mut Tally) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Native {
    Gmime,
    Vmime,
    Libetpan,
    Dovecot,
}

static GMIME_INIT: Once = Once::new();
static DOVECOT_INIT: Once = Once::new();

impl Native {
    fn entry(self, workload: Workload) -> Entry {
        match (self, workload) {
            (Native::Gmime, Workload::Structure) => cmp_gmime_structure,
            (Native::Gmime, Workload::Headers) => cmp_gmime_headers,
            (Native::Gmime, Workload::Full) => cmp_gmime_full,
            (Native::Vmime, Workload::Structure) => cmp_vmime_structure,
            (Native::Vmime, Workload::Headers) => cmp_vmime_headers,
            (Native::Vmime, Workload::Full) => cmp_vmime_full,
            (Native::Libetpan, Workload::Structure) => cmp_libetpan_structure,
            (Native::Libetpan, Workload::Headers) => cmp_libetpan_headers,
            (Native::Libetpan, Workload::Full) => cmp_libetpan_full,
            (Native::Dovecot, Workload::Structure) => cmp_dovecot_structure,
            (Native::Dovecot, Workload::Headers) => cmp_dovecot_headers,
            (Native::Dovecot, Workload::Full) => cmp_dovecot_full,
        }
    }

    fn init(self) {
        match self {
            Native::Gmime => GMIME_INIT.call_once(|| unsafe { cmp_gmime_init() }),
            Native::Dovecot => DOVECOT_INIT.call_once(|| unsafe { cmp_dovecot_init() }),
            Native::Vmime | Native::Libetpan => {}
        }
    }
}

pub fn run(native: Native, workload: Workload, raw: &[u8], tally: &mut Tally) -> bool {
    native.init();
    let entry = native.entry(workload);
    let raw = black_box(raw);
    unsafe { entry(raw.as_ptr(), raw.len(), tally) != 0 }
}
