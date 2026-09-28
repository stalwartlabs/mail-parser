/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{after, is_field_end};

pub(crate) fn dash_line(hay: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?
        .windows(3)
        .position(|window| window == b"\n--")
        .map(|pos| pos + from)
}

pub(crate) fn field_end(hay: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .map(|(pos, _)| pos + from)
        .find(|&pos| is_field_end(after(hay, pos)))
}
