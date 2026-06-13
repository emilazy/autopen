// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Unix socket peer attestation for Linux.

use std::{
    ffi::c_void,
    os::fd::{AsFd, AsRawFd as _, OwnedFd},
};

use camino::Utf8PathBuf;
use color_eyre::eyre::{self, WrapErr as _, eyre};
use pidfd_util::{PidFd, PidFdExt as _};
use tokio::io;
use tokio::net::UnixStream;

/// Attestation for a Unix socket peer.
#[derive(Debug)]
pub(crate) struct PeerAttestation {
    /// The path of the control group the remote peer of a socket
    /// belongs to.
    #[expect(dead_code, reason = "currently used only for tracing")]
    cgroup_path: Utf8PathBuf,
}

/// Gets an attestation for the remote peer of a socket.
///
/// # Errors
///
/// Returns an error if information about the peer cannot be obtained.
#[tracing::instrument(level = tracing::Level::DEBUG)]
pub(crate) fn attest_peer(conn: &UnixStream) -> eyre::Result<PeerAttestation> {
    Ok(PeerAttestation {
        cgroup_path: attest_peer_process_cgroup_path(conn)
            .wrap_err("Failed to obtain peer process control cgroup")?,
    })
}

// TODO: Should we open the cgroup path and keep it open, in case the
// path gets recycled? (It’s not clear what we’d do in that case,
// though; we’re fundamentally just attesting that the process was in
// the control group at some point.)
/// Gets the path of the control group the remote peer of the socket
/// belongs to.
///
/// # Errors
///
/// Returns an error if information about the peer process cannot be
/// obtained because of kernel errors or environmental issues (e.g.
/// `/proc` being inaccessible or cgroup v2 being unavailable).
fn attest_peer_process_cgroup_path(conn: &UnixStream) -> eyre::Result<Utf8PathBuf> {
    let peer_pidfd: PidFd = socket_peerpidfd(conn)
        .wrap_err("Failed to obtain peer process ID file descriptor")?
        .into();
    // TODO: Can we get the cgroup ID and access through that instead? (It
    // seemed like there’d be some thorny permissions issues with
    // `open_by_handle_at(2)`.)
    let peer_proc_cgroup_contents = peer_pidfd
        .access_proc(|| {
            let peer_pid = peer_pidfd
                .get_pid()
                .wrap_err("Failed to obtain peer process ID")?;
            let peer_proc_cgroup_path = Utf8PathBuf::from(format!("/proc/{peer_pid}/cgroup"));
            // We assume that reads to `/proc` won’t meaningfully block.
            std::fs::read_to_string(&peer_proc_cgroup_path)
                .wrap_err_with(|| format!("Failed to read {peer_proc_cgroup_path}"))
        })
        .wrap_err("Failed to read peer process information")??;
    let mut peer_proc_cgroup_lines = peer_proc_cgroup_contents.lines();
    let (Some(peer_process_cgroup_path), None) = (
        peer_proc_cgroup_lines
            .next()
            .and_then(|line| line.strip_prefix("0::"))
            .map(Utf8PathBuf::from),
        peer_proc_cgroup_lines.next(),
    ) else {
        return Err(eyre!(
            "Failed to parse peer process cgroup file as cgroup v2 format"
        ));
    };
    Ok(peer_process_cgroup_path)
}

// TODO: Replace this with rustix once
// <https://github.com/bytecodealliance/rustix/pull/1474> lands.
/// Gets a PID file descriptor for the remote peer of the socket.
///
/// # Errors
///
/// Forwards errors on from `getsockopt(2)`.
#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "no rustix binding for this interface exists yet"
)]
fn socket_peerpidfd<Fd: AsFd>(fd: Fd) -> io::Result<OwnedFd> {
    let borrowed_fd = fd.as_fd();
    let mut peer_pidfd = None;
    #[expect(clippy::missing_panics_doc, reason = "invariant")]
    let mut peer_pidfd_size = size_of_val(&peer_pidfd)
        .try_into()
        .expect("the size of a file descriptor should fit into socklen_t");
    // SAFETY: The file descriptor is valid, `peer_pidfd` is
    // initialized and valid for writes of size `peer_pidfd_size`,
    // and `peer_pidfd_size` is valid for writes.
    let result = unsafe {
        libc::getsockopt(
            borrowed_fd.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERPIDFD,
            (&raw mut peer_pidfd).cast::<c_void>(),
            &raw mut peer_pidfd_size,
        )
    };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        #[expect(clippy::missing_panics_doc, reason = "invariant")]
        Ok(peer_pidfd.expect("a successful SO_PEERPIDFD call should return a file descriptor"))
    }
}
