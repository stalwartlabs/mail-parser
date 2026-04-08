/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "cli")]
fn main() {
    cli::run();
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("Build with `--features cli` to enable the CLI");
}
