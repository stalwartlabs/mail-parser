/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod address;
mod body;
mod builder;
mod header;
mod message;
#[cfg(feature = "rkyv")]
pub mod rkyv;
