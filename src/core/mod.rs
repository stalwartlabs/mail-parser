/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

pub mod address;
pub mod body;
pub mod builder;
pub mod header;
mod html;
pub mod message;
#[cfg(feature = "rkyv")]
pub mod rkyv;

pub use html::Html;
