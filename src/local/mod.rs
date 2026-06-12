// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Local contexts.

pub(crate) mod restorer;
mod serialize;

use std::{
    fmt::{self, Debug},
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
};
pub(crate) use serialize::{Secrecy, Serialize, SerializeFile, save};

/// The bootstrap interface for a local context.
pub(crate) struct Bootstrap {
    /// The local reference restorer.
    restorer: restorer::Client,
}

impl Bootstrap {
    /// Creates a new `Bootstrap` for a local context.
    pub(crate) fn new() -> Self {
        Self {
            restorer: capnp_rpc::new_client(Restorer::new()),
        }
    }

    /// Returns a client for the local reference restorer.
    pub(crate) const fn restorer(&self) -> &restorer::Client {
        &self.restorer
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
        f.debug_struct("Bootstrap").finish_non_exhaustive()
    }
}
