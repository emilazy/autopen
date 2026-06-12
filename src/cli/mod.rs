// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The implementation of the `autopen(1)` command‐line interface.

mod sign;
mod signing_key;
mod verify;
mod x509;

use clap::Parser as _;
use color_eyre::eyre::{self, WrapErr as _};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::local::Bootstrap;

#[doc(hidden)]
pub fn main() -> eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_location_section(false)
        .display_env_section(false)
        .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
        .issue_filter(|kind| matches!(kind, color_eyre::ErrorKind::NonRecoverable(_)))
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .install()
        .expect("`color_eyre` hook should install successfully");

    let cli = Autopen::parse();

    tracing_subscriber::registry()
        .with(tracing_error::ErrorLayer::default())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .parse(&cli.global_options.log)
                .wrap_err("Failed to parse log filter directives")?,
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .without_time()
                .with_target(false),
        )
        .init();

    debug!(?cli, "Executing command");
    tokio::runtime::LocalRuntime::new()
        .wrap_err("Failed to create Tokio runtime")?
        .block_on(cli.command.run(Bootstrap::new()))
}

/// A cryptographic signing service with an object‐capability interface.
#[derive(Debug, clap::Parser)]
#[command(version)]
struct Autopen {
    /// The global options for this invocation.
    #[command(flatten, next_help_heading = "Global options")]
    global_options: GlobalOptions,

    /// The subcommand to execute.
    #[command(subcommand)]
    command: Command,
}

/// Options shared by all subcommands.
#[derive(Debug, clap::Args)]
struct GlobalOptions {
    /// Log filter directives for tracing.
    ///
    /// For a complete syntax reference, see:
    /// <https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/filter/struct.EnvFilter.html#directives>.
    #[arg(
        global = true,
        long,
        env = "AUTOPEN_LOG",
        value_name = "DIRECTIVES",
        default_value = "info"
    )]
    log: String,
}

/// Subcommands of `autopen(1)`.
#[cfg_attr(
    // Work around <https://github.com/rust-lang/rust-clippy/issues/16934>.
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "subcommands are documented by their respective types"
    )
)]
#[derive(Debug, clap::Subcommand)]
enum Command {
    Sign(sign::Command),
    Verify(verify::Command),
    #[command(subcommand)]
    SigningKey(signing_key::Command),
    #[command(subcommand)]
    X509(x509::Command),
}

/// An `autopen(1)` subcommand.
pub(crate) trait Subcommand: clap::FromArgMatches {
    /// Run the subcommand.
    ///
    /// The implementation should be annotated with
    /// `#[tracing::instrument(level = tracing::Level::DEBUG)]` unless
    /// it simply dispatches to subcommands itself. The caller is
    /// expected to have set up an `eyre` hook and a `tracing`
    /// subscriber as necessary.
    ///
    /// # Errors
    ///
    /// Returns an [`eyre::Report`] for any user‐facing errors.
    async fn run(self, local: Bootstrap) -> eyre::Result<()>;
}

impl Subcommand for Command {
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        match self {
            Self::Sign(cmd) => cmd.run(local).await,
            Self::Verify(cmd) => cmd.run(local).await,
            Self::SigningKey(cmd) => cmd.run(local).await,
            Self::X509(cmd) => cmd.run(local).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory as _;

    use super::*;

    #[test]
    fn clap_debug_assert() {
        Autopen::command().debug_assert();
    }
}
