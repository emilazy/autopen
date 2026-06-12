// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Local restorer for remote references.

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Debug},
};

use camino::Utf8PathBuf;
use capnp::{
    capability::{FromClientHook as _, Rc},
    message,
};
use capnp_futures::io::tokio::UnixFdStream;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::io::VatNetwork};
use tokio::{fs::File, net::UnixStream, task};
use tracing::{debug, error};

use crate::autopen_capnp::{
    bootstrap,
    local::remote_ref,
    restorer,
    unix_socket_server::{self, file_ref},
};

/// A client for a local reference restorer.
pub(crate) type Client = restorer::Client<remote_ref::Owned>;

/// A local reference restorer.
#[derive(Debug, Default)]
pub(crate) struct Restorer {
    /// Connections to remote servers.
    remotes: RefCell<Remotes>,
}

// TODO: We need a better story here around cleaning up connections and
// retrying failed ones.

/// A map of remote server addresses to clients for their
/// bootstrap interfaces.
#[derive(Clone, Default)]
struct Remotes(HashMap<Utf8PathBuf, unix_socket_server::Client>);

impl Debug for Remotes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.0.keys()).finish()
    }
}

impl Restorer {
    /// Returns a new restorer with no active connections.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns a client for a remote server’s bootstrap interface,
    /// creating a new connection if necessary.
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    fn remote(self: &Rc<Self>, socket_path: Utf8PathBuf) -> unix_socket_server::Client {
        self.remotes
            .borrow_mut()
            .0
            .entry(socket_path)
            .or_insert_with_key(|key| capnp_rpc::new_future_client(connect(key.clone())))
            .clone()
    }
}

impl restorer::Server<remote_ref::Owned> for Restorer {
    #[tracing::instrument(level = tracing::Level::INFO, skip(params, results), err)]
    async fn restore(
        self: Rc<Self>,
        params: restorer::RestoreParams<remote_ref::Owned>,
        mut results: restorer::RestoreResults<remote_ref::Owned>,
    ) -> capnp::Result<()> {
        let remote_ref = params.get()?.get_sturdy_ref()?;
        let socket_path = Utf8PathBuf::from(remote_ref.get_socket_path()?.to_str()?);
        let remote = self.remote(socket_path);
        let file_ref_path = Utf8PathBuf::from(remote_ref.get_file_ref_path()?.to_str()?);
        let mut request = remote
            .cast_to::<bootstrap::Client<file_ref::Owned>>()
            .get_restorer_request()
            .send()
            .pipeline
            .get_restorer()
            .restore_request();
        request
            .get()
            .set_sturdy_ref(capnp_rpc::new_future_client(async move {
                let file_ref: file_ref::Client =
                    capnp_rpc::new_client(File::open(file_ref_path).await?);
                Ok(file_ref)
            }))?;
        let mut results = results.get();
        results
            .reborrow()
            .init_cap()
            .set_as_capability(request.send().pipeline.get_cap().as_cap());
        debug!(results = ?results.into_reader());
        Ok(())
    }
}

/// Connects to a remote server and returns a client for its
/// bootstrap interface.
///
/// # Errors
///
/// Returns an error if the connection fails.
#[tracing::instrument(level = tracing::Level::INFO, err)]
async fn connect(socket_path: Utf8PathBuf) -> capnp::Result<unix_socket_server::Client> {
    debug!("Connecting…");
    let conn = UnixStream::connect(&socket_path).await.map_err(|err| {
        capnp::Error::disconnected(format!("Failed to connect to {socket_path}: {err}"))
    })?;
    debug!(?conn, "Connected");
    let (rx, tx) = conn.into_split();
    let network = VatNetwork::new_with_fds(
        // TODO: This should use buffering once that’s possible with
        // FD passing.
        UnixFdStream::new(rx),
        UnixFdStream::new(tx),
        1,
        Side::Client,
        message::ReaderOptions::default(),
    );
    let mut rpc_system = RpcSystem::new(Box::new(network), None);
    let client = rpc_system.bootstrap(Side::Server);
    task::spawn_local(async {
        if let Err(err) = rpc_system.await {
            error!(error = %err, "RPC system failed");
        }
    });
    Ok(client)
}
