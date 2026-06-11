// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen verify` subcommand.

use camino::Utf8PathBuf;
use color_eyre::eyre::{self, WrapErr as _};
use tokio::fs;

use crate::{
    cli::Subcommand,
    local::Bootstrap,
    verification_key::{VerificationKey, Verifier as _},
};

/// Verify a message against a signature and verification key.
#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    /// The message to verify the signature for.
    message: Utf8PathBuf,
    /// The verification key to use.
    #[arg(long, value_name = "PATH")]
    verification_key: Utf8PathBuf,
    /// A purported signature to verify against the message and key.
    #[arg(long, value_name = "PATH")]
    signature: Utf8PathBuf,
}

impl Subcommand for Command {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let message = fs::read(&self.message)
            .await
            .wrap_err_with(|| format!("Failed to read message file {}", self.message))?;
        let verification_key: VerificationKey = local.load(&self.verification_key).await?;
        let signature = fs::read(&self.signature)
            .await
            .wrap_err_with(|| format!("Failed to read signature file {}", self.signature))?;
        verification_key.verify(&message, &signature)?;
        Ok(())
    }
}
