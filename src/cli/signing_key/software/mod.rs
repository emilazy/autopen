// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen signing-key software` subcommand.

mod rsa3072_pkcs1_sha256;

use color_eyre::eyre;

use crate::{cli::Subcommand, local::Bootstrap};

/// Commands for software signing keys.
#[cfg_attr(
    // Work around <https://github.com/rust-lang/rust-clippy/issues/16934>.
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "subcommands are documented by their respective types"
    )
)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    #[command(subcommand)]
    Rsa3072Pkcs1Sha256(rsa3072_pkcs1_sha256::Command),
}

impl Subcommand for Command {
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        match self {
            Self::Rsa3072Pkcs1Sha256(cmd) => cmd.run(local).await,
        }
    }
}
