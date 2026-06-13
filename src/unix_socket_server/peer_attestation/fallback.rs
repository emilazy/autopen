// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Unix socket peer attestation stub for unsupported platforms.

use color_eyre::eyre;
use tokio::net::UnixStream;

/// A fallback Unix socket peer attestation implementation for
/// unsupported platforms.
#[derive(Debug)]
pub(crate) struct PeerAttestation;

/// Returns a fallback Unix socket peer attestation structure.
///
/// # Errors
///
/// Never returns an error.
#[tracing::instrument(level = tracing::Level::DEBUG)]
pub(crate) fn attest_peer(conn: &UnixStream) -> eyre::Result<PeerAttestation> {
    Ok(PeerAttestation)
}
