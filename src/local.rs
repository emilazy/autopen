// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Local contexts.

/// The bootstrap interface for a local context.
#[derive(Debug)]
pub(crate) struct Bootstrap;

impl Bootstrap {
    /// Creates a new `Bootstrap` for a local context.
    pub(crate) const fn new() -> Self {
        Self
    }
}
