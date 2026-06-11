// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Local contexts.

mod serialize;

use camino::Utf8Path;
use color_eyre::eyre;

use crate::autopen_capnp::local;
pub(crate) use serialize::{Secrecy, Serialize, SerializeFile, save};

/// The bootstrap interface for a local context.
#[derive(Debug)]
pub(crate) struct Bootstrap;

impl Bootstrap {
    /// Creates a new `Bootstrap` for a local context.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Load a [`SerializeFile`] implementer from a persisted file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or
    /// deserialization fails.
    pub(crate) async fn load<T: SerializeFile>(&self, path: &Utf8Path) -> eyre::Result<T> {
        serialize::load(path).await
    }
}

impl local::Server for Bootstrap {}
