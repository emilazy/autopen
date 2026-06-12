// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! An implementation of out‐of‐band detached signing for
//! [`rcgen::SigningKey`].

use std::cell::{Cell, OnceCell};
use std::fmt::Debug;

use rcgen::PublicKeyData as _;
use tracing::debug;

use crate::verification_key::{Verifier, VerifyError};

/// A verification key that can be used as an X.509 subject public key.
pub(crate) trait SubjectPublicKey: Verifier {
    /// Returns the signature algorithm the verification key is
    /// used for.
    fn algorithm(&self) -> &'static rcgen::SignatureAlgorithm;

    /// Converts the verification key to the X.509 `subjectPublicKey`
    /// format appropriate for this signature algorithm.
    fn to_subject_public_key(&self) -> Vec<u8>;
}

impl<K: SubjectPublicKey + ?Sized> SubjectPublicKey for &K {
    fn algorithm(&self) -> &'static rcgen::SignatureAlgorithm {
        (*self).algorithm()
    }

    fn to_subject_public_key(&self) -> Vec<u8> {
        (*self).to_subject_public_key()
    }
}

/// A verification key bundled with its precomputed X.509
/// `subjectPublicKey` bytes.
#[derive(Debug)]
pub(crate) struct SubjectPublicKeyInfo<K> {
    /// The verification key.
    verification_key: K,
    /// The X.509 `subjectPublicKey` bytes corresponding to
    /// `verification_key`.
    subject_public_key: Vec<u8>,
}

impl<K: SubjectPublicKey> SubjectPublicKeyInfo<K> {
    /// Creates a new `SubjectPublicKeyInfo` for the given
    /// verification key.
    pub(crate) fn new(verification_key: K) -> Self {
        Self {
            subject_public_key: verification_key.to_subject_public_key(),
            verification_key,
        }
    }

    /// Encodes the verification key into the ASN.1 DER encoding of a
    /// `SubjectPublicKeyInfo`, as defined in [Section 4.1 of
    /// RFC 5280].
    ///
    /// [RFC 5280]: <https://www.rfc-editor.org/info/rfc5280/#section-4.1>
    pub(crate) fn to_der(&self) -> Vec<u8> {
        self.subject_public_key_info()
    }
}

impl<K: SubjectPublicKey> rcgen::PublicKeyData for SubjectPublicKeyInfo<K> {
    fn der_bytes(&self) -> &[u8] {
        &self.subject_public_key
    }

    fn algorithm(&self) -> &'static rcgen::SignatureAlgorithm {
        self.verification_key.algorithm()
    }
}

/// An [`rcgen::SigningKey`] that expects a single message to be
/// requested for signing and extracts it for signing out of band.
#[derive(Debug)]
pub(crate) struct MessageDetacher<K> {
    /// The subject public key to sign for.
    subject_public_key_info: SubjectPublicKeyInfo<K>,
    /// A cell to hold the message to be signed.
    message: OnceCell<Vec<u8>>,
}

impl<K: SubjectPublicKey> MessageDetacher<K> {
    /// Creates a new `MessageDetacher` with the given verification key.
    pub(crate) fn new(verification_key: K) -> Self {
        Self {
            subject_public_key_info: SubjectPublicKeyInfo::new(verification_key),
            message: OnceCell::new(),
        }
    }

    /// Consumes the `MessageDetacher`, returning the message to sign,
    /// or `None` if no signing request was received.
    pub(crate) fn into_message(self) -> Option<Vec<u8>> {
        self.message.into_inner()
    }
}

impl<K: SubjectPublicKey> rcgen::PublicKeyData for MessageDetacher<K> {
    fn der_bytes(&self) -> &[u8] {
        self.subject_public_key_info.der_bytes()
    }

    fn algorithm(&self) -> &'static rcgen::SignatureAlgorithm {
        self.subject_public_key_info.algorithm()
    }
}

impl<K: SubjectPublicKey + Debug> rcgen::SigningKey for MessageDetacher<K> {
    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        ret,
        err(level = tracing::Level::DEBUG),
    )]
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, rcgen::Error> {
        match self.message.set(msg.to_owned()) {
            Ok(()) => Ok(vec![]),
            Err(_) => Err(rcgen::Error::RemoteKeyError),
        }
    }
}

/// An [`rcgen::SigningKey`] that expects a single message to be
/// requested for signing and returns a corresponding signature that
/// was generated out of band.
pub(crate) struct SignatureAttacher<K> {
    /// The subject public key to sign for.
    subject_public_key_info: SubjectPublicKeyInfo<K>,
    /// A cell to hold the out‐of‐band signature.
    signature: Cell<Option<Vec<u8>>>,
}

impl<K: SubjectPublicKey> SignatureAttacher<K> {
    /// Creates a new `SignatureAttacher` with the given verification
    /// key and signature.
    pub(crate) fn new(verification_key: K, signature: Vec<u8>) -> Self {
        Self {
            subject_public_key_info: SubjectPublicKeyInfo::new(verification_key),
            signature: Cell::new(Some(signature)),
        }
    }
}

impl<K: SubjectPublicKey> rcgen::PublicKeyData for SignatureAttacher<K> {
    fn der_bytes(&self) -> &[u8] {
        self.subject_public_key_info.der_bytes()
    }

    fn algorithm(&self) -> &'static rcgen::SignatureAlgorithm {
        self.subject_public_key_info.algorithm()
    }
}

impl<K: SubjectPublicKey + Debug> rcgen::SigningKey for SignatureAttacher<K> {
    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        ret,
        err(level = tracing::Level::DEBUG),
    )]
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, rcgen::Error> {
        let signature = self.signature.take().ok_or(rcgen::Error::RemoteKeyError)?;
        debug!(signature = &signature[..]);
        match self
            .subject_public_key_info
            .verification_key
            .verify(msg, &signature)
        {
            Ok(()) => Ok(signature),
            Err(VerifyError) => Err(rcgen::Error::RemoteKeyError),
        }
    }
}

impl<K: Debug> Debug for SignatureAttacher<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignatureAttacher")
            .field("subject_public_key_info", &self.subject_public_key_info)
            .finish_non_exhaustive()
    }
}
