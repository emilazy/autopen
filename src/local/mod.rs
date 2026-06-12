// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Local contexts.

pub(crate) mod restorer;
mod serialize;

use std::{
    fmt::{self, Debug},
    mem,
    rc::Rc,
};

use camino::Utf8Path;
use color_eyre::eyre;

use crate::{
    autopen_capnp::{
        bootstrap,
        local::{self, remote_ref},
    },
    local::restorer::Restorer,
    socket_activation::ReceivedSockets,
};
pub(crate) use serialize::{Secrecy, Serialize, SerializeFile, save};

/// The bootstrap interface for a local context.
pub(crate) struct Bootstrap {
    /// The local reference restorer.
    restorer: restorer::Client,
    /// The file descriptors that were received as part of
    /// socket activation.
    sockets: ReceivedSockets,
}

impl Bootstrap {
    /// Creates a new `Bootstrap` for a local context.
    pub(crate) fn new(sockets: ReceivedSockets) -> Self {
        Self {
            restorer: capnp_rpc::new_client(Restorer::new()),
            sockets,
        }
    }

    /// Returns a client for the local reference restorer.
    pub(crate) const fn restorer(&self) -> &restorer::Client {
        &self.restorer
    }

    /// Takes the sockets received from the environment out.
    pub(crate) fn take_sockets(&mut self) -> ReceivedSockets {
        mem::take(&mut self.sockets)
    }

    /// Load a [`SerializeFile`] implementer from a persisted file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or
    /// deserialization fails.
    pub(crate) async fn load<T: SerializeFile>(&self, path: &Utf8Path) -> eyre::Result<T> {
        serialize::load(self.restorer(), path).await
    }
}

impl bootstrap::Server<remote_ref::Owned> for Bootstrap {
    async fn get_restorer(
        self: Rc<Self>,
        _params: bootstrap::GetRestorerParams<remote_ref::Owned>,
        mut results: bootstrap::GetRestorerResults<remote_ref::Owned>,
    ) -> capnp::Result<()> {
        results.get().set_restorer(self.restorer().clone());
        Ok(())
    }
}

impl local::Server for Bootstrap {}

impl Debug for Bootstrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bootstrap")
            .field("sockets", &self.sockets)
            .finish_non_exhaustive()
    }
}
