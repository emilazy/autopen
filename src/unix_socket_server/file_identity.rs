// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! POSIX file identities.

use std::{
    marker::PhantomData,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    panic,
};

use rustix::fs;
use tokio::{fs::File, io, task};

/// An owned file descriptor that is open for reading, with a known
/// [`FileIdentifier`].
#[derive(Debug)]
pub(crate) struct KnownReadableFile {
    /// The underlying file description.
    fd: OwnedFd,
    /// The identifier for the file.
    ///
    /// This is conceptually borrowed from `fd`.
    identifier: FileIdentifier<'static>,
}

/// An identifier for a file.
///
/// If two file descriptions have the same identifier, then they refer
/// to the same underlying file.
///
/// This value can be compared and hashed freely. However, note that
/// these comparisons and hashes are only valid for the given lifetime;
/// the operating system may reuse the underlying identifier values
/// once the corresponding file is no longer open (for instance,
/// because it was deleted and had its identifier recycled for use by a
/// new file).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FileIdentifier<'a> {
    /// The `st_dev` field from the file’s `struct stat` metadata.
    dev: fs::Dev,
    /// The `st_ino` field from the file’s `struct stat` metadata.
    ino: u64,
    /// Per the POSIX.1‐2024 [`<sys/stat.h>` documentation]:
    ///
    /// [`<sys/stat.h>` documentation]:
    /// <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_stat.h.html>
    ///
    /// > A *file identity* is uniquely determined by the combination
    /// > of *st_dev* and *st_ino*. At any given time in a system,
    /// > distinct files shall have distinct file identities; hard
    /// > links to the same file shall have the same file identity.
    /// > Over time, these file identities can be reused for different
    /// > files. For example, the *st_ino* value can be reused after
    /// > the last link to a file is unlinked and the space occupied by
    /// > the file has been freed, and the *st_dev* value associated
    /// > with a file system can be reused if that file system is
    /// > detached ("unmounted") and another is attached ("mounted").
    ///
    /// Therefore, the combination of `dev` and `ino` is a file
    /// identifier with the properties we require, but it is only valid
    /// as long as the underlying file exists. This is guaranteed if
    /// there is an open file descriptor referring to it.
    _marker: PhantomData<BorrowedFd<'a>>,
}

impl KnownReadableFile {
    /// Wraps a readable file with its queried file identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not opened for reading or if
    /// querying its identity fails.
    #[tracing::instrument(level = tracing::Level::INFO)]
    pub(crate) async fn from_file(file: File) -> io::Result<Self> {
        Self::from_fd(file.into_std().await.into()).await
    }

    /// Wraps a readable file descriptor with its queried
    /// file identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not opened for reading or if
    /// querying its identity fails.
    #[tracing::instrument(level = tracing::Level::INFO, err)]
    pub(crate) async fn from_fd(fd: OwnedFd) -> io::Result<Self> {
        task::spawn_blocking(move || {
            let flags = fs::fcntl_getfl(&fd)?;
            let acc_mode = flags & fs::OFlags::ACCMODE;
            if acc_mode != fs::OFlags::RDONLY && acc_mode != fs::OFlags::RDWR {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            // Per <https://github.com/bytecodealliance/rustix/blob/v1.1.4/src/backend/libc/fs/types.rs#L318-L326>.
            #[cfg(any(
                target_os = "android",
                target_os = "linux",
                target_os = "emscripten",
                target_os = "freebsd",
                target_os = "fuchsia",
                target_os = "redox",
            ))]
            if flags.contains(fs::OFlags::PATH) {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            let stat = fs::fstat(&fd)?;
            Ok(Self {
                fd,
                identifier: FileIdentifier {
                    dev: stat.st_dev,
                    ino: stat.st_ino,
                    _marker: PhantomData,
                },
            })
        })
        .await
        .unwrap_or_else(|err| panic::resume_unwind(err.into_panic()))
    }

    /// Returns the file identifier.
    pub(crate) const fn identifier(&self) -> FileIdentifier<'_> {
        self.identifier
    }
}

impl AsFd for KnownReadableFile {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for KnownReadableFile {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl From<KnownReadableFile> for OwnedFd {
    fn from(file: KnownReadableFile) -> Self {
        file.fd
    }
}
