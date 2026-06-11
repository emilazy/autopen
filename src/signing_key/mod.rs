// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Signing keys.

pub(crate) mod software;

use crate::{
    autopen_capnp::{local::file, signer, signing_key},
    local::{Serialize, SerializeFile},
};

/// A signing key.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum SigningKey {
    /// A software signing key.
    Software(software::SigningKey),
}

impl SigningKey {
    /// Converts the signing key into a [`signer::Client`] to use the
    /// Cap’n Proto `Signer` interface with.
    pub(crate) fn into_signer(self) -> signer::Client {
        match self {
            Self::Software(key) => key.into_signer(),
        }
    }
}

impl Serialize for SigningKey {
    type Owned = signing_key::Owned;

    fn read_capnp(reader: signing_key::Reader<'_>) -> capnp::Result<Self> {
        match reader.which()? {
            signing_key::Software(reader) => Ok(software::SigningKey::read_capnp(reader?)?.into()),
            signing_key::Reserved(()) => Err(capnp::NotInSchema(1).into()),
        }
    }

    fn build_capnp(&self, builder: signing_key::Builder<'_>) -> capnp::Result<()> {
        match self {
            Self::Software(key) => key.build_capnp(builder.init_software()),
        }
    }
}

impl SerializeFile for SigningKey {
    const NAME: &'static str = "signing key";

    fn get_from_file(reader: file::Reader<'_>) -> Option<capnp::Result<signing_key::Reader<'_>>> {
        match reader.which() {
            Ok(file::SigningKey(reader)) => Some(reader),
            _ => None,
        }
    }

    fn init_in_file(builder: file::Builder<'_>) -> signing_key::Builder<'_> {
        builder.init_signing_key()
    }
}

impl From<software::SigningKey> for SigningKey {
    fn from(key: software::SigningKey) -> Self {
        Self::Software(key)
    }
}
