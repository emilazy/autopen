// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen signing-key software rsa3072-pkcs1-sha256` subcommand.

use camino::Utf8PathBuf;
use color_eyre::eyre::{self, WrapErr as _};

use crate::{
    cli::Subcommand,
    local::{self, Bootstrap, Secrecy},
    signing_key::{SigningKey, software::rsa3072_pkcs1_sha256},
};

/// Commands for software `rsa3072-pkcs1-sha256` signing keys.
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
    Generate(Generate),
}

impl Subcommand for Command {
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        match self {
            Self::Generate(cmd) => cmd.run(local).await,
        }
    }
}

/// Generate a new `rsa3072-pkcs1-sha256` software signing key.
///
/// **Warning:** This is not side‐channel safe, and should not be used
/// in untrusted multi‐tenant or physical environments.
#[derive(Debug, clap::Args)]
pub(crate) struct Generate {
    /// The file to write the generated signing key to.
    #[arg(long, value_name = "PATH")]
    output: Utf8PathBuf,
}

impl Subcommand for Generate {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let signing_key = SigningKey::Software(
            rsa3072_pkcs1_sha256::SigningKey::generate()
                .wrap_err("Failed to generate signing key")?
                .into(),
        );
        local::save(&self.output, &signing_key, Secrecy::Secret).await?;
        Ok(())
    }
}
