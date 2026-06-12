// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Local persistent serialization.

use camino::Utf8Path;
use capnp::{message, serialize::OwnedSegments, traits::Owned};
use capnp_futures::io::{AsyncFdReadExt as _, serialize, tokio::Compat};
use color_eyre::eyre::{self, WrapErr as _};
use tokio::fs::File;

use crate::{autopen_capnp::local::file, local::restorer};

/// A type that can be serialized to and deserialized from a Cap’n
/// Proto structure.
pub(crate) trait Serialize: Sized {
    /// The corresponding Cap’n Proto type.
    type Owned: Owned;

    /// Reads a serialized Cap’n Proto structure.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn read_capnp(
        restorer: &restorer::Client,
        reader: <Self::Owned as Owned>::Reader<'_>,
    ) -> capnp::Result<Self>;

    /// Builds a serialized Cap’n Proto structure.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn build_capnp(&self, builder: <Self::Owned as Owned>::Builder<'_>) -> capnp::Result<()>;
}

/// A type that can be stored as the top‐level entity in a
/// persisted file.
pub(crate) trait SerializeFile: Serialize {
    /// A human‐readable name for the type.
    const NAME: &'static str;

    /// Gets the reader for the type’s union variant.
    fn get_from_file(
        reader: file::Reader<'_>,
    ) -> Option<capnp::Result<<Self::Owned as Owned>::Reader<'_>>>;

    /// Initializes a builder for the type’s union variant.
    fn init_in_file(builder: file::Builder<'_>) -> <Self::Owned as Owned>::Builder<'_>;
}

// TODO: Consider whether we should be using the packed or
// canonical format for files, and whether buffering would be a
// good idea.

/// Load a [`SerializeFile`] implementer from a persisted file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or
/// deserialization fails.
pub(crate) async fn load<T: SerializeFile>(
    restorer: &restorer::Client,
    path: &Utf8Path,
) -> eyre::Result<T> {
    load_file(path)
        .await
        .and_then(|message| {
            T::read_capnp(
                restorer,
                T::get_from_file(message.get()?)
                    .ok_or_else(|| capnp::Error::failed(format!("Not a {}", T::NAME)))??,
            )
        })
        .wrap_err_with(|| format!("Failed to read {} file {path}", T::NAME))
}

/// Load a raw `Local.File` structure from a persisted file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or
/// deserialization fails.
async fn load_file(
    path: &Utf8Path,
) -> capnp::Result<message::TypedReader<OwnedSegments, file::Owned>> {
    let mut file = Compat::new(File::open(path).await?);
    let message = serialize::read_message(&mut file, message::ReaderOptions::default()).await?;
    if file.read(&mut [0]).await? != 0 {
        return Err(capnp::Error::failed(
            "Trailing bytes after message".to_owned(),
        ));
    }
    Ok(message.into_typed())
}

/// Save a [`SerializeFile`] implementer to a persisted file.
///
/// # Errors
///
/// Returns an error if the file cannot be written or
/// serialization fails.
pub(crate) async fn save<T: SerializeFile>(
    path: &Utf8Path,
    value: &T,
    secrecy: Secrecy,
) -> eyre::Result<()> {
    let mut builder = message::TypedBuilder::new_default();
    match value.build_capnp(T::init_in_file(builder.init_root())) {
        Ok(()) => save_file(path, builder, secrecy).await,
        Err(err) => Err(err),
    }
    .wrap_err_with(|| format!("Failed to write {} file {path}", T::NAME))
}

/// Save a raw `Local.File` structure to a persisted file.
///
/// # Errors
///
/// Returns an error if the file cannot be written or
/// serialization fails.
async fn save_file(
    path: &Utf8Path,
    value: message::TypedBuilder<file::Owned>,
    secrecy: Secrecy,
) -> capnp::Result<()> {
    let file = File::options()
        .write(true)
        .create_new(true)
        .mode(secrecy.to_mode())
        .open(path)
        .await?;
    serialize::write_message(Compat::new(file), value.into_inner()).await
}

/// The secrecy of a file being written.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Secrecy {
    /// The file is public information.
    Public,
    /// The file contains or represents privileged capabilities and
    /// should only be accessible by the invoking user.
    Secret,
}

impl Secrecy {
    /// Returns the appropriate file mode bits for the secrecy.
    const fn to_mode(self) -> u32 {
        match self {
            Self::Public => 0o666,
            Self::Secret => 0o600,
        }
    }
}
