// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! A Unix socket server that provides access to signing keys.

mod file_identity;

use std::{
    fmt::{self, Debug},
    hash,
    os::fd::{AsFd as _, BorrowedFd},
    rc::Rc,
};

use capnp::{
    capability::{self, FromClientHook},
    message,
};
use capnp_futures::io::tokio::UnixFdStream;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::io::VatNetwork};
use color_eyre::eyre::{self, WrapErr as _};
use iddqd::{IdHashItem, IdHashMap, id_upcast};
use tokio::{fs::File, io, net::UnixStream};
use tracing::{debug, info};

use crate::{
    autopen_capnp::{
        bootstrap, restorer,
        unix_socket_server::{self, file_ref},
    },
    unix_socket_server::file_identity::KnownReadableFile,
};

/// The bootstrap interface for a Unix socket server.
pub(crate) struct Bootstrap {
    /// The server’s file reference restorer.
    restorer: restorer::Client<file_ref::Owned>,
}

impl Bootstrap {
    /// Creates a new server with the given file references.
    pub(crate) fn new(refs: IdHashMap<KnownRef, hash::RandomState>) -> Self {
        Self {
            restorer: capnp_rpc::new_client(Restorer { refs }),
        }
    }

    /// Serve a connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the Cap’n Proto RPC system returns an error.
    #[tracing::instrument(level = tracing::Level::INFO, ret, err)]
    pub(crate) async fn serve(self: Rc<Self>, conn: UnixStream) -> eyre::Result<()> {
        info!("New connection");

        let (rx, tx) = conn.into_split();
        let network = VatNetwork::new_with_fds(
            // TODO: This should use buffering once that’s possible with
            // FD passing.
            UnixFdStream::new(rx),
            UnixFdStream::new(tx),
            1,
            Side::Server,
            message::ReaderOptions::default(),
        );

        let client: unix_socket_server::Client = capnp_rpc::new_client_from_rc(self);
        RpcSystem::new(Box::new(network), Some(client.client))
            .await
            .wrap_err("RPC system failed")?;
        Ok(())
    }
}

impl bootstrap::Server<file_ref::Owned> for Bootstrap {
    async fn get_restorer(
        self: Rc<Self>,
        _params: bootstrap::GetRestorerParams<file_ref::Owned>,
        mut results: bootstrap::GetRestorerResults<file_ref::Owned>,
    ) -> capnp::Result<()> {
        results.get().set_restorer(self.restorer.clone());
        Ok(())
    }
}

impl unix_socket_server::Server for Bootstrap {}

impl Debug for Bootstrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bootstrap").finish_non_exhaustive()
    }
}

/// A restorer for sturdy references based on file identities.
#[derive(Debug)]
struct Restorer {
    /// The known file references.
    refs: IdHashMap<KnownRef, hash::RandomState>,
}

impl restorer::Server<file_ref::Owned> for Restorer {
    #[tracing::instrument(level = tracing::Level::INFO, skip(params, results), err)]
    async fn restore(
        self: Rc<Self>,
        params: restorer::RestoreParams<file_ref::Owned>,
        mut results: restorer::RestoreResults<file_ref::Owned>,
    ) -> capnp::Result<()> {
        // TODO: Since we use a single‐threaded runtime, a single file
        // descriptor that blocks on close (e.g. because of NFS or
        // FUSE) would hang the entire server process. This isn’t
        // relevant for the Nix use case, but it’s unfortunate
        // nonetheless. Sadly there’s not much we can do to make this
        // work smoothly in the general case due to OS limitations, but
        // it might at least be be a good idea to arrange for the file
        // descriptors to be closed on another thread.
        let file = params.get()?.get_sturdy_ref()?;
        let fd =
            file.client.get_fd().await?.ok_or_else(|| {
                capnp::Error::failed("No file descriptor was received".to_owned())
            })?;
        let fd = fd.try_clone_to_owned().map_err(|_err| {
            capnp::Error::overloaded("Failed to clone file descriptor".to_owned())
        })?;
        let known_file = KnownReadableFile::from_fd(fd)
            .await
            .map_err(|_err| capnp::Error::failed("Failed to identify file".to_owned()))?;
        debug!(?known_file);
        let cap = self
            .refs
            .get(&known_file.identifier())
            // TODO: Should this use `capnp::Error::disconnected` instead?
            .ok_or_else(|| capnp::Error::failed("Requested reference does not exist".to_owned()))?
            .cap
            .as_client_hook()
            .add_ref();
        let mut results = results.get();
        results.reborrow().init_cap().set_as_capability(cap);
        debug!(results = ?results.into_reader());
        Ok(())
    }
}

/// A file reference and its corresponding capability.
pub(crate) struct KnownRef {
    /// The file reference.
    file: KnownReadableFile,
    /// The capability the file reference should resolve to.
    cap: capability::Client,
}

impl KnownRef {
    /// Create a file reference record mapping `file` to `cap`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not opened for reading or if
    /// querying its identity fails.
    pub(crate) async fn new<T: FromClientHook>(file: File, cap: T) -> io::Result<Self> {
        Ok(Self {
            file: KnownReadableFile::from_file(file).await?,
            cap: cap.cast_to(),
        })
    }
}

impl IdHashItem for KnownRef {
    type Key<'a> = file_identity::FileIdentifier<'a>;

    fn key(&self) -> Self::Key<'_> {
        self.file.identifier()
    }

    id_upcast!();
}

impl Debug for KnownRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KnownRef")
            .field("file", &self.file)
            .finish_non_exhaustive()
    }
}

impl file_ref::Server for File {
    fn get_fd(&self) -> Option<BorrowedFd<'_>> {
        Some(self.as_fd())
    }
}
