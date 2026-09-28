/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(dead_code)]

use mail_parser::{MessagePart, MessageRef};
use std::{fs, path::PathBuf};

const SNAPSHOT_SUITES: [&str; 5] = ["rfc", "legacy", "thirdparty", "malformed", "regressions"];

pub fn resource(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/eml")
            .join(path),
    )
    .expect("fixture exists")
}

pub fn fixture(name: &str) -> Vec<u8> {
    resource(&format!("issues/{name}"))
}

pub fn regression(name: &str) -> Vec<u8> {
    resource(&format!("regressions/{name}.eml"))
}

pub fn line_endings(raw: &[u8]) -> [(Vec<u8>, &'static str); 2] {
    let lf: Vec<u8> = raw.iter().copied().filter(|&byte| byte != b'\r').collect();
    let mut crlf = Vec::with_capacity(lf.len() * 2);
    for &byte in &lf {
        if byte == b'\n' {
            crlf.push(b'\r');
        }
        crlf.push(byte);
    }
    [(lf, "\n"), (crlf, "\r\n")]
}

pub fn ids<'m>(parts: impl Iterator<Item = MessagePart<'m>>) -> Vec<u32> {
    parts.map(|part| part.id()).collect()
}

pub fn lists(message: MessageRef<'_>) -> [Vec<u32>; 3] {
    [
        ids(message.text_body()),
        ids(message.html_body()),
        ids(message.attachments()),
    ]
}

pub fn snapshot_messages() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for suite in SNAPSHOT_SUITES {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/eml")
            .join(suite);
        let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("the suite directory exists")
            .map(|entry| entry.expect("the directory is readable").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "eml"))
            .collect();
        assert!(!paths.is_empty(), "no message in {}", dir.display());
        paths.sort_unstable();
        found.extend(paths);
    }
    found
}

pub fn snapshot_inputs(raw: &[u8]) -> [(Vec<u8>, &'static str); 2] {
    let lf = raw.iter().copied().filter(|&byte| byte != b'\r').collect();
    let mut crlf = Vec::with_capacity(raw.len() + raw.len() / 16);
    let mut last = 0;
    for &byte in raw {
        if byte == b'\n' && last != b'\r' {
            crlf.push(b'\r');
        }
        crlf.push(byte);
        last = byte;
    }
    [(lf, "json"), (crlf, "crlf.json")]
}
