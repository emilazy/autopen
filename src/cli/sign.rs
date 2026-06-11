// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen sign` subcommand.

use camino::Utf8PathBuf;
use color_eyre::eyre::{self, WrapErr as _};
use tokio::fs;

use crate::{
    autopen_capnp::signer::sign_results, cli::Subcommand, local::Bootstrap, signing_key::SigningKey,
};

/// Sign a message with a signing key.
#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    /// The signing key to use.
    #[arg(long, value_name = "PATH")]
    signing_key: Utf8PathBuf,
    /// The file to write the signature to.
    #[arg(long, value_name = "PATH")]
    output: Utf8PathBuf,
    /// The message to sign.
    message: Utf8PathBuf,
}

impl Subcommand for Command {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let signing_key: SigningKey = local.load(&self.signing_key).await?;
        let message = fs::read(&self.message)
            .await
            .wrap_err_with(|| format!("Failed to read message file {}", self.message))?;
        let mut request = signing_key.into_signer().sign_request();
        request.get().set_message(&message);
        let response = request
            .send()
            .promise
            .await
            .wrap_err("Failed to sign message")?;
        let signature = response
            .get()
            .and_then(sign_results::Reader::get_signature)
            .wrap_err("Failed to read signature")?;
        fs::write(&self.output, &signature)
            .await
            .wrap_err("Failed to write signature")?;
        Ok(())
    }
}
