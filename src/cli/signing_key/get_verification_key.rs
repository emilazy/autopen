// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen signing-key get-verification-key` subcommand.

use camino::Utf8PathBuf;
use color_eyre::eyre::{self, WrapErr as _};

use crate::{
    cli::Subcommand,
    local::{self, Bootstrap, Secrecy, Serialize as _},
    signing_key::SigningKey,
    verification_key::VerificationKey,
};

/// Write the verification key corresponding to a signing key.
#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    /// The signing key.
    #[arg(long, value_name = "PATH")]
    signing_key: Utf8PathBuf,
    /// The file to write the verification key to.
    #[arg(long, value_name = "PATH")]
    output: Utf8PathBuf,
}

impl Subcommand for Command {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let signing_key: SigningKey = local.load(&self.signing_key).await?;
        let response = signing_key
            .into_signer()
            .get_verification_key_request()
            .send()
            .promise
            .await
            .wrap_err("Failed to request verification key")?;
        let verification_key = response
            .get()
            .and_then(|results| {
                VerificationKey::read_capnp(local.restorer(), results.get_verification_key()?)
            })
            .wrap_err("Failed to read verification key")?;
        local::save(&self.output, &verification_key, Secrecy::Public).await?;
        Ok(())
    }
}
