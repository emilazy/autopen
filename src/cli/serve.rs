// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen serve` subcommand.

use std::rc::Rc;

use camino::Utf8PathBuf;
use color_eyre::eyre::{self, WrapErr as _, eyre};
use iddqd::IdHashMap;
use tokio::{fs::File, net::UnixListener, signal, task::JoinSet};
use tracing::{error, info};

use crate::{cli::Subcommand, local::Bootstrap, signing_key::SigningKey, unix_socket_server};

/// Provide access to signing keys over a socket.
///
/// `autopen signing-key remote create` creates signing keys that will
/// sign indirectly through a server.
#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    /// The path to bind the server socket to.
    #[arg(long, value_name = "PATH")]
    socket_path: Utf8PathBuf,
    /// Map a file reference to a signing key.
    #[arg(
        long = "signing-key-ref",
        num_args = 2,
        value_names = ["FILE_REF_PATH", "SIGNING_KEY_PATH"],
    )]
    signing_key_refs: Vec<Vec<Utf8PathBuf>>,
}

impl Subcommand for Command {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let mut refs = IdHashMap::default();
        for key_ref in self.signing_key_refs {
            let [file_ref_path, signing_key_path] =
                key_ref.try_into().expect("clap should enforce num_args");
            let file = File::open(&file_ref_path)
                .await
                .wrap_err_with(|| format!("Failed to open file reference {file_ref_path}"))?;
            let signing_key: SigningKey = local.load(&signing_key_path).await?;
            let signer = signing_key.into_signer();
            let known_ref = unix_socket_server::KnownRef::new(file, signer)
                .await
                .wrap_err_with(|| format!("Failed to identify file reference {file_ref_path}"))?;
            refs.insert_unique(known_ref).map_err(|_err| {
                eyre!("File reference {file_ref_path} was mapped multiple times")
            })?;
        }
        let server = Rc::new(unix_socket_server::Bootstrap::new(refs));

        let listener = UnixListener::bind(&self.socket_path)
            .wrap_err_with(|| format!("Failed to listen on {}", self.socket_path))?;

        info!("Listening for connections…");
        let mut tasks = JoinSet::new();
        let mut shutting_down = false;
        loop {
            tokio::select! {
                result = listener.accept(), if !shutting_down => {
                    let (conn, _) = result.wrap_err("Failed to accept connection")?;
                    let server = Rc::clone(&server);
                    tasks.spawn_local(async move {
                        // Errors are reported by `tracing::instrument`.
                        let _err = server.serve(conn).await;
                    });
                }
                () = shutdown_signal(), if !shutting_down => {
                    info!("Shutting down…");
                    shutting_down = true;
                }
                Some(result) = tasks.join_next() => {
                    if let Err(err) = result && err.is_panic() {
                        error!(error = %err, "Connection task panicked");
                    }
                }
                else => break,
            }
        }
        Ok(())
    }
}

/// Waits for a Ctrl+C or `SIGTERM` signal.
///
/// # Panics
///
/// Panics if a handler fails to install.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("`ctrl_c` handler should register successfully");
    };

    let terminate = async {
        _ = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("`SIGTERM` handler should register successfully")
            .recv()
            .await;
    };

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    };
}
