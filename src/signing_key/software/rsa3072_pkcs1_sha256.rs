// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! `rsa3072-pkcs1-sha256` software signing keys.
//!
//! See [`crate::verification_key::rsa3072_pkcs1_sha256`] for
//! more information.

use std::{
    fmt::{self, Debug},
    rc::Rc,
};

use graviola::signing::rsa;
use tracing::{debug, error, record_all};

use crate::{
    autopen_capnp::{signer, signing_key::software::rsa3072_pkcs1_sha256},
    local::Serialize,
    verification_key::{self, rsa3072_pkcs1_sha256::VerificationKey},
};

/// An RSA signing key with a 3072‐bit modulus.
pub(crate) struct SigningKey(Box<rsa::SigningKey>);

impl SigningKey {
    /// Generates a new signing key.
    ///
    /// **Warning:** This is not side‐channel safe, and should not be
    /// used in untrusted multi‐tenant or physical environments.
    ///
    /// # Errors
    ///
    /// Forwards errors on from the underlying key generation method.
    pub(crate) fn generate() -> Result<Self, graviola::Error> {
        Ok(Self(Box::new(rsa::SigningKey::generate(
            rsa::KeySize::Rsa3072,
        )?)))
    }

    /// Decodes a signing key from the ASN.1 DER encoding of an
    /// `RSAPrivateKey`, as defined in [Appendix A.1.2 of RFC 8017].
    ///
    /// [Appendix A.1.2 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.2>
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails or the modulus size is not
    /// 3072 bits.
    fn from_pkcs1_der(bytes: &[u8]) -> Result<Self, graviola::Error> {
        let signing_key = Box::new(rsa::SigningKey::from_pkcs1_der(bytes)?);
        (signing_key.modulus_len_bytes() == MODULUS_LEN_BYTES)
            .then_some(Self(signing_key))
            .ok_or(graviola::Error::OutOfRange)
    }

    /// Encodes the signing key into the ASN.1 DER encoding of an
    /// `RSAPrivateKey`, as defined in [Appendix A.1.2 of RFC 8017].
    ///
    /// [Appendix A.1.2 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.2>
    fn to_pkcs1_der<'buf>(&self, buf: &'buf mut [u8; PKCS1_DER_MAX_LEN]) -> &'buf [u8] {
        #[expect(clippy::missing_panics_doc, reason = "invariant")]
        self.0
            .to_pkcs1_der(buf)
            .expect("encoding should fit into PKCS1_DER_MAX_LEN bytes")
    }
}

/// The expected size of the modulus in bits.
const MODULUS_LEN_BITS: usize = 3072;

/// The expected size of the modulus in bytes.
const MODULUS_LEN_BYTES: usize = MODULUS_LEN_BITS / 8;

/// The maximum size in bytes of an ASN.1 DER encoding of an
/// `RSAPrivateKey`, as defined in [Appendix A.1.2 of RFC 8017], with
/// two primes, a 3072‐bit modulus, and a public exponent of 65537.
///
/// [Appendix A.1.2 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.2>
pub(crate) const PKCS1_DER_MAX_LEN: usize = 1769;

impl signer::Server for SigningKey {
    #[tracing::instrument(
        level = tracing::Level::INFO,
        skip(params, results),
        fields(params.message),
        err,
    )]
    async fn sign(
        self: Rc<Self>,
        params: signer::SignParams,
        mut results: signer::SignResults,
    ) -> capnp::Result<()> {
        let message = params.get()?.get_message()?;
        record_all!(tracing::Span::current(), params.message = message);
        let mut results = results.get();
        let buf = results.reborrow().init_signature(
            MODULUS_LEN_BYTES
                .try_into()
                .expect("MODULUS_LEN_BYTES should fit into u32"),
        );
        if let Err(err) = self.0.sign_pkcs1_sha256(buf, message) {
            error!(error = %err);
            return Err(capnp::Error::failed("Failed to sign message".to_owned()));
        }
        debug!(results = ?results.into_reader());
        Ok(())
    }

    #[tracing::instrument(level = tracing::Level::INFO, skip(_params, results), err)]
    async fn get_verification_key(
        self: Rc<Self>,
        _params: signer::GetVerificationKeyParams,
        mut results: signer::GetVerificationKeyResults,
    ) -> capnp::Result<()> {
        let mut results = results.get();
        verification_key::VerificationKey::from(VerificationKey::from_graviola(
            self.0.public_key(),
        ))
        .build_capnp(results.reborrow().init_verification_key())?;
        debug!(results = ?results.into_reader());
        Ok(())
    }
}

impl Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SigningKey").finish_non_exhaustive()
    }
}

impl Serialize for SigningKey {
    type Owned = rsa3072_pkcs1_sha256::Owned;

    fn read_capnp(reader: rsa3072_pkcs1_sha256::Reader<'_>) -> capnp::Result<Self> {
        Self::from_pkcs1_der(reader.get_pkcs1_der()?)
            .map_err(|err| capnp::Error::failed(err.to_string()))
    }

    fn build_capnp(&self, mut builder: rsa3072_pkcs1_sha256::Builder<'_>) -> capnp::Result<()> {
        builder.set_pkcs1_der(self.to_pkcs1_der(&mut [0; _]));
        Ok(())
    }
}
