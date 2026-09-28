/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{FieldCtx, trim_fws};
use crate::store::{Str, Value};
use std::ops::Range;

pub(crate) fn parse_raw(ctx: &mut FieldCtx<'_>, value: Range<usize>) -> Value {
    let trimmed = ctx.trim_fws(value);
    if trimmed.is_empty() {
        Value::Text(Str::EMPTY)
    } else {
        Value::Text(ctx.borrow(trimmed))
    }
}

pub(crate) fn borrowed(src: &[u8], value: Range<usize>) -> Option<Value> {
    let trimmed = trim_fws(src, value);
    if trimmed.is_empty() {
        Some(Value::Text(Str::EMPTY))
    } else {
        Str::borrow(src, trimmed).map(Value::Text)
    }
}
