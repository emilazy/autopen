// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The main `autopen(1)` executable entry point.

#![expect(
    unused_crate_dependencies,
    reason = "https://github.com/rust-lang/rust/issues/95513"
)]

use color_eyre::eyre;

#[cfg_attr(
    test,
    expect(
        clippy::missing_errors_doc,
        reason = "https://github.com/rust-lang/rust-clippy/issues/14491"
    )
)]
fn main() -> eyre::Result<()> {
    autopen::main()
}
