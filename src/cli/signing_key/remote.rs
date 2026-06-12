// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen signing-key remote` subcommand.

use camino::Utf8PathBuf;
use capnp::message;
use color_eyre::eyre::{self, WrapErr as _};

use crate::{
    autopen_capnp::local::remote_ref,
    cli::Subcommand,
    local::{self, Bootstrap, Secrecy},
    signing_key::{SigningKey, remote},
};

/// Commands for remote signing keys.
///
/// These are stubs that reference signing keys accessible through
/// a remote server (see `autopen serve`).
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
    Create(Create),
}

impl Subcommand for Command {
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        match self {
            Self::Create(cmd) => cmd.run(local).await,
        }
    }
}

/// Create a remote signing key stub.
#[derive(Debug, clap::Args)]
pub(crate) struct Create {
    /// The path to the server socket.
    #[arg(long, value_name = "PATH")]
    socket_path: Utf8PathBuf,
    /// The file to send to the server as a key reference.
    #[arg(long, value_name = "PATH")]
    file_ref_path: Utf8PathBuf,
    /// The verification key corresponding to the remote signing key.
    #[arg(long, value_name = "PATH")]
    verification_key: Utf8PathBuf,
    /// The file to write the generated signing key to.
    #[arg(long, value_name = "PATH")]
    output: Utf8PathBuf,
}

impl Subcommand for Create {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let verification_key = local.load(&self.verification_key).await?;
        let mut builder = message::Builder::new_default();
        let mut remote_ref: remote_ref::Builder<'_> = builder.init_root();
        remote_ref.set_socket_path(self.socket_path);
        remote_ref.set_file_ref_path(self.file_ref_path);
        let signing_key: SigningKey =
            remote::SigningKey::new(local.restorer(), remote_ref.into_reader(), verification_key)
                .wrap_err("Failed to create remote signing key")?
                .into();
        local::save(&self.output, &signing_key, Secrecy::Secret).await?;
        Ok(())
    }
}
