// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen signing-key` subcommand.

mod get_verification_key;
mod remote;
mod software;

use color_eyre::eyre;

use crate::{cli::Subcommand, local::Bootstrap};

/// Commands for signing keys.
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
    Software(software::Command),
    #[command(subcommand)]
    Remote(remote::Command),
    GetVerificationKey(get_verification_key::Command),
}

impl Subcommand for Command {
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        match self {
            Self::Software(cmd) => cmd.run(local).await,
            Self::Remote(cmd) => cmd.run(local).await,
            Self::GetVerificationKey(cmd) => cmd.run(local).await,
        }
    }
}
