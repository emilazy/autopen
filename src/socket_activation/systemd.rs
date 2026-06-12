// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Socket activation with the systemd protocol.

#![expect(
    unsafe_code,
    reason = "https://github.com/rust-lang/rust/issues/116059"
)]

use std::fmt::{self, Debug};

use color_eyre::eyre::{self, WrapErr as _, eyre};
use listen_fds::{ListenFds, ListenFdsError};
use tokio::net::UnixListener;

/// A set of file descriptors received from the environment.
#[derive(Default)]
pub(crate) struct ReceivedSockets(Option<ListenFds>);

impl Debug for ReceivedSockets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceivedSockets").finish_non_exhaustive()
    }
}

impl ReceivedSockets {
    /// Receives file descriptors from the environment according to the
    /// [`sd_listen_fds(3)`] protocol.
    ///
    /// [`sd_listen_fds(3)`]:
    /// <https://www.freedesktop.org/software/systemd/man/latest/sd_listen_fds.html>
    ///
    /// # Safety
    ///
    /// This removes variables from the environment, so all the safety
    /// requirements of [`std::env::remove_var()`] apply.
    ///
    /// Additionally, [`std::os::fd::FromRawFd::from_raw_fd()`] is
    /// called on raw file descriptors computed based on environment
    /// variables. Therefore, either the contents of the relevant
    /// environment variables must be trustworthy, or no file
    /// descriptors in the process above `SD_LISTEN_FDS_START` may have
    /// an existing owner.
    ///
    /// In practice, this should be called early in `main`, before
    /// opening any files or spawning any threads. Unfortunately, even
    /// that is not necessarily a guarantee in the presence of native
    /// libraries. See [Rust issue #116059] for some discussion of the
    /// issues here.
    ///
    /// [Rust issue #116059]: <https://github.com/rust-lang/rust/issues/116059>
    ///
    /// # Errors
    ///
    /// Returns an error if the `sd_listen_fds(3)` protocol is
    /// violated.
    pub(crate) unsafe fn receive() -> eyre::Result<Self> {
        // SAFETY: The caller has asserted the safety requirements we
        // pass on.
        let result = unsafe { ListenFds::new() };
        let inner = match result {
            Err(ListenFdsError::NoListenFds | ListenFdsError::PidMissmatch { .. }) => None,
            _ => Some(
                result.wrap_err("Failed to receive file descriptors from the service manager")?,
            ),
        };
        Ok(Self(inner))
    }

    /// Takes a [`UnixListener`] with the given `name` out from the
    /// received file descriptors.
    ///
    /// # Errors
    ///
    /// Returns an error if there is more than one file descriptor with
    /// the given name, or if there is a problem converting it into a
    /// [`UnixListener`].
    pub(crate) fn take_unix_listener(
        &mut self,
        name: &'static str,
    ) -> eyre::Result<Option<UnixListener>> {
        if let Some(fds) = self.0.as_mut() {
            let mut iter = fds.take(name);
            let Some(fd) = iter.next() else {
                return Ok(None);
            };
            let None = iter.next() else {
                return Err(eyre!("Too many sockets named {name}"));
            };
            #[expect(
                clippy::absolute_paths,
                reason = "disambiguation with the Tokio version"
            )]
            let listener = std::os::unix::net::UnixListener::from(fd);
            listener
                .set_nonblocking(true)
                .wrap_err("Failed to move socket into non‐blocking mode")?;
            Ok(Some(
                UnixListener::from_std(listener).wrap_err("Failed to create listener")?,
            ))
        } else {
            Ok(None)
        }
    }
}
