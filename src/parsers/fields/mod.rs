/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

pub mod address;
pub mod content_type;
pub mod date;
pub mod id;
pub mod list;
pub mod raw;
pub mod received;
pub mod thread;
pub mod unstructured;

#[cfg(test)]
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Test<T> {
    pub header: String,
    pub expected: T,
}

#[cfg(test)]
pub fn load_tests<T: serde::de::DeserializeOwned>(test_name: &str) -> Vec<Test<T>> {
    serde_json::from_slice::<Vec<Test<T>>>(
        &std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join(test_name),
        )
        .unwrap(),
    )
    .unwrap()
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct TestBuilder<T: Serialize> {
    test: std::path::PathBuf,
    tests: Vec<Test<T>>,
}

#[cfg(test)]
impl<T: Serialize> TestBuilder<T> {
    pub fn new(test: impl AsRef<str>) -> Self {
        Self {
            test: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join(test.as_ref()),
            tests: Vec::new(),
        }
    }

    pub fn add(&mut self, header: impl Into<String>, expected: T) {
        self.tests.push(Test {
            header: header.into(),
            expected,
        });
    }

    pub fn write(&self) {
        std::fs::write(
            &self.test,
            serde_json::to_string_pretty(&self.tests).unwrap(),
        )
        .unwrap();
    }
}
