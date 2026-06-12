// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Socket activation stub for unsupported platforms.

use color_eyre::eyre;
use tokio::net::UnixListener;

/// A fallback socket activation implementation for
/// unsupported platforms.
#[derive(Debug, Default)]
pub(crate) struct ReceivedSockets;

impl ReceivedSockets {
    /// Returns a fallback socket activation structure.
    ///
    /// # Safety
    ///
    /// This implementation is always safe to call, but is marked
    /// `unsafe` due to the safety requirements of other
    /// implementations of the interface. Callers must satisfy the
    /// safety requirements of all other implementations.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    #[expect(unsafe_code, reason = "interface compatibility")]
    #[expect(clippy::missing_const_for_fn, reason = "interface compatibility")]
    #[expect(clippy::unnecessary_wraps, reason = "interface compatibility")]
    pub(crate) unsafe fn receive() -> eyre::Result<Self> {
        Ok(Self)
    }

    /// Returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    #[expect(clippy::missing_const_for_fn, reason = "interface compatibility")]
    #[expect(clippy::needless_pass_by_ref_mut, reason = "interface compatibility")]
    #[expect(clippy::unnecessary_wraps, reason = "interface compatibility")]
    #[expect(clippy::unused_self, reason = "interface compatibility")]
    pub(crate) fn take_unix_listener(
        &mut self,
        _name: &'static str,
    ) -> eyre::Result<Option<UnixListener>> {
        Ok(None)
    }
}
