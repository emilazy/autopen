// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Software signing keys.

pub(crate) mod rsa3072_pkcs1_sha256;

use crate::{
    autopen_capnp::{signer, signing_key::software},
    local::{Serialize, restorer},
};

/// A software signing key.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum SigningKey {
    /// A software `rsa3072-pkcs1-sha256` signing key.
    Rsa3072Pkcs1Sha256(rsa3072_pkcs1_sha256::SigningKey),
}

impl SigningKey {
    /// Converts the signing key into a [`signer::Client`] to use the
    /// Cap’n Proto `Signer` interface with.
    pub(crate) fn into_signer(self) -> signer::Client {
        match self {
            Self::Rsa3072Pkcs1Sha256(key) => capnp_rpc::new_client(key),
        }
    }
}

impl Serialize for SigningKey {
    type Owned = software::Owned;

    fn read_capnp(
        restorer: &restorer::Client,
        reader: software::Reader<'_>,
    ) -> capnp::Result<Self> {
        match reader.which()? {
            software::Rsa3072Pkcs1Sha256(reader) => {
                Ok(rsa3072_pkcs1_sha256::SigningKey::read_capnp(restorer, reader?)?.into())
            }
            software::Reserved(()) => Err(capnp::NotInSchema(1).into()),
        }
    }

    fn build_capnp(&self, builder: software::Builder<'_>) -> capnp::Result<()> {
        match self {
            Self::Rsa3072Pkcs1Sha256(key) => key.build_capnp(builder.init_rsa3072_pkcs1_sha256()),
        }
    }
}

impl From<rsa3072_pkcs1_sha256::SigningKey> for SigningKey {
    fn from(key: rsa3072_pkcs1_sha256::SigningKey) -> Self {
        Self::Rsa3072Pkcs1Sha256(key)
    }
}
