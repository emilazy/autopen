// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Verification keys.

pub(crate) mod rsa3072_pkcs1_sha256;

use std::{
    error::Error,
    fmt::{self, Display},
};

use crate::{
    autopen_capnp::{local::file, verification_key},
    local::{Serialize, SerializeFile},
};

/// A verification key.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum VerificationKey {
    /// An `rsa3072-pkcs1-sha256` verification key.
    Rsa3072Pkcs1Sha256(rsa3072_pkcs1_sha256::VerificationKey),
}

impl Serialize for VerificationKey {
    type Owned = verification_key::Owned;

    fn read_capnp(reader: verification_key::Reader<'_>) -> capnp::Result<Self> {
        match reader.which()? {
            verification_key::Rsa3072Pkcs1Sha256(reader) => {
                Ok(rsa3072_pkcs1_sha256::VerificationKey::read_capnp(reader?)?.into())
            }
            verification_key::Reserved(()) => Err(capnp::NotInSchema(1).into()),
        }
    }

    fn build_capnp(&self, builder: verification_key::Builder<'_>) -> capnp::Result<()> {
        match self {
            Self::Rsa3072Pkcs1Sha256(key) => key.build_capnp(builder.init_rsa3072_pkcs1_sha256()),
        }
    }
}

impl SerializeFile for VerificationKey {
    const NAME: &'static str = "verification key";

    fn get_from_file(
        reader: file::Reader<'_>,
    ) -> Option<capnp::Result<verification_key::Reader<'_>>> {
        match reader.which() {
            Ok(file::VerificationKey(reader)) => Some(reader),
            _ => None,
        }
    }

    fn init_in_file(builder: file::Builder<'_>) -> verification_key::Builder<'_> {
        builder.init_verification_key()
    }
}

impl From<rsa3072_pkcs1_sha256::VerificationKey> for VerificationKey {
    fn from(key: rsa3072_pkcs1_sha256::VerificationKey) -> Self {
        Self::Rsa3072Pkcs1Sha256(key)
    }
}

/// An abstract verification key.
pub(crate) trait Verifier {
    /// Verify `signature` against the given `message`.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is invalid.
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), VerifyError>;
}

/// A signature verification error.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct VerifyError;

impl Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to verify signature")
    }
}

impl Error for VerifyError {}

impl<K: Verifier + ?Sized> Verifier for &K {
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), VerifyError> {
        (*self).verify(message, signature)
    }
}

impl Verifier for VerificationKey {
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), VerifyError> {
        match self {
            Self::Rsa3072Pkcs1Sha256(key) => key.verify(message, signature),
        }
    }
}
