// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! `rsa3072-pkcs1-sha256` verification keys.
//!
//! This is `RSASSA-PKCS1-v1_5` as defined in [Section 8.2 of RFC
//! 8017], using RSA keys with 3072‐bit moduli and SHA‐256 as the hash
//! function.
//!
//! [Section 8.2 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#section-8.2>

use std::fmt::{self, Debug};

use graviola::signing::rsa;

use crate::{
    autopen_capnp::verification_key::rsa3072_pkcs1_sha256,
    local::Serialize,
    verification_key::{Verifier, VerifyError},
    x509,
};

/// An RSA verification key with a 3072‐bit modulus.
pub(crate) struct VerificationKey(Box<rsa::VerifyingKey>);

impl VerificationKey {
    /// Creates a verification key from an [`rsa::VerifyingKey`].
    pub(crate) fn from_graviola(inner: rsa::VerifyingKey) -> Self {
        // TODO: Verify key size.
        Self(Box::new(inner))
    }

    /// Decodes a verification key from the ASN.1 DER encoding of an
    /// `RSAPublicKey`, as defined in [Appendix A.1.1 of RFC 8017].
    ///
    /// [Appendix A.1.1 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.1>
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails.
    fn from_pkcs1_der(bytes: &[u8; PKCS1_DER_LEN]) -> Result<Self, graviola::Error> {
        Ok(Self::from_graviola(rsa::VerifyingKey::from_pkcs1_der(
            &bytes[..],
        )?))
    }

    /// Encodes the verification key into the ASN.1 DER encoding of an
    /// `RSAPublicKey`, as defined in [Appendix A.1.1 of RFC 8017].
    ///
    /// [Appendix A.1.1 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.1>
    #[expect(clippy::missing_panics_doc, reason = "invariant")]
    fn to_pkcs1_der(&self, buf: &mut [u8; PKCS1_DER_LEN]) {
        assert_eq!(
            self.0.to_pkcs1_der(buf).map(<[u8]>::len),
            Ok(PKCS1_DER_LEN),
            "encoding should be PKCS1_DER_LEN bytes",
        );
    }
}

/// The size in bytes of an ASN.1 DER encoding of an
/// `RSAPublicKey`, as defined in [Appendix A.1.1 of RFC 8017], with a
/// 3072‐bit modulus and a public exponent of 65537.
///
/// [Appendix A.1.1 of RFC 8017]: <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.1>
pub(crate) const PKCS1_DER_LEN: usize = 398;

impl Verifier for VerificationKey {
    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        skip(message),
        fields(msg = message),
        ret,
        err(level = tracing::Level::DEBUG),
    )]
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), VerifyError> {
        self.0
            .verify_pkcs1_sha256(signature, message)
            .map_err(|_err| VerifyError)
    }
}

impl x509::SubjectPublicKey for VerificationKey {
    fn algorithm(&self) -> &'static rcgen::SignatureAlgorithm {
        &rcgen::PKCS_RSA_SHA256
    }

    fn to_subject_public_key(&self) -> Vec<u8> {
        let mut buf = [0; _];
        self.to_pkcs1_der(&mut buf);
        buf.to_vec()
    }
}

impl Debug for VerificationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0; _];
        self.to_pkcs1_der(&mut buf);
        f.debug_tuple("VerificationKey::from_pkcs1_der")
            .field(&buf)
            .finish()
    }
}

impl Serialize for VerificationKey {
    type Owned = rsa3072_pkcs1_sha256::Owned;

    fn read_capnp(reader: rsa3072_pkcs1_sha256::Reader<'_>) -> capnp::Result<Self> {
        reader
            .get_pkcs1_der()?
            .as_array()
            .ok_or(graviola::Error::OutOfRange)
            .and_then(Self::from_pkcs1_der)
            .map_err(|err| capnp::Error::failed(err.to_string()))
    }

    fn build_capnp(&self, builder: rsa3072_pkcs1_sha256::Builder<'_>) -> capnp::Result<()> {
        let buf = builder
            .init_pkcs1_der(
                PKCS1_DER_LEN
                    .try_into()
                    .expect("PKCS1_DER_LEN should fit into u32"),
            )
            .as_mut_array()
            .expect("builder should be the requested size");
        self.to_pkcs1_der(buf);
        Ok(())
    }
}
