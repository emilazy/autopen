// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Utilities for creating X.509 certificates.

use std::fmt::Debug;

use color_eyre::eyre::{self, OptionExt as _, eyre};
use graviola::hashing::{Hash as _, Sha256};

use crate::x509::signature::{
    MessageDetacher, SignatureAttacher, SubjectPublicKey, SubjectPublicKeyInfo,
};

/// An X.509 certificate builder.
#[derive(Debug)]
pub(crate) struct CertificateBuilder<K> {
    /// The verification key to use as the certificate subject’s
    /// public key.
    pub verification_key: K,
    /// The purpose for which the public key is certified for use.
    pub purpose: KeyPurpose,
    /// The certificate subject’s common name.
    pub common_name: String,
    /// The certificate’s validity period.
    pub validity_period: CertificateValidityPeriod,
}

/// A purpose for which a public key can be certified for use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, clap::ValueEnum)]
pub(crate) enum KeyPurpose {
    /// Signing of executable code.
    CodeSigning,
}

/// A certificate validity period between 1 second and 365
/// days (inclusive).
#[derive(Debug)]
pub(crate) struct CertificateValidityPeriod {
    /// The beginning of the certificate’s validity period.
    not_before: time::UtcDateTime,
    /// The end of the certificate’s validity period (inclusive).
    not_after: time::UtcDateTime,
}

impl CertificateValidityPeriod {
    /// Returns a certificate validity period from a beginning and
    /// a duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the certificate validity period is too
    /// short, too long, or results in an overflow.
    pub(crate) fn new(
        not_before: time::UtcDateTime,
        lifetime: time::Duration,
    ) -> eyre::Result<Self> {
        if !(time::Duration::seconds(1)..=time::Duration::days(365)).contains(&lifetime) {
            return Err(eyre!("Invalid certificate validity period"));
        }
        Ok(Self {
            not_before,
            not_after: not_before
                .checked_add(
                    // Per Section 4.1.2.5 of RFC 5280:
                    //
                    // > The validity period for a certificate is the period of time from
                    // > notBefore through notAfter, inclusive.
                    #[expect(clippy::missing_panics_doc, reason = "invariant")]
                    lifetime
                        .checked_sub(time::Duration::seconds(1))
                        .expect("certificate lifetime should already have been validated"),
                )
                .ok_or_eyre("Certificate validity period resulted in overflow")?,
        })
    }
}

impl<K: SubjectPublicKey + Debug> CertificateBuilder<K> {
    /// Consumes the `CertificateBuilder` and returns the verification
    /// key and the computed [`rcgen::CertificateParams`].
    fn prepare(self) -> (K, rcgen::CertificateParams) {
        let Self {
            verification_key,
            purpose,
            common_name,
            validity_period,
        } = self;
        let mut params = rcgen::CertificateParams::default();
        match purpose {
            KeyPurpose::CodeSigning => {
                params
                    .key_usages
                    .push(rcgen::KeyUsagePurpose::DigitalSignature);
                params.insert_extended_key_usage(rcgen::ExtendedKeyUsagePurpose::CodeSigning);
            }
        }
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, common_name);
        params.not_before = validity_period.not_before.into();
        params.not_after = validity_period.not_after.into();
        params.serial_number = {
            // TODO: This matches what rcgen does out of the box, but
            // results in duplicate serial numbers if two distinct
            // certificates are created for the same verification key.
            // It’s bad practice to reuse verification keys anyway, but
            // it would be nice to do something more robust here.
            //
            // The CA/B Baseline Requirements require the use of actual
            // entropy here, but we’d like deterministic generation. A
            // hash of the certificate parameters or a user‐supplied
            // seed are probably the best options.
            let mut serial_number = truncated_sha256(&verification_key.to_subject_public_key());
            serial_number[0] &= 0x7F;
            Some(rcgen::SerialNumber::from_slice(&serial_number))
        };
        params.is_ca = rcgen::IsCa::ExplicitNoCa;
        params.key_identifier_method = rcgen::KeyIdMethod::PreSpecified(
            rfc7093_truncated_sha256_key_identifier(&verification_key).to_vec(),
        );
        params.use_authority_key_identifier_extension = true;
        (verification_key, params)
    }

    /// Prepares a certificate for self‐signing, and returns the ASN.1
    /// DER encoding of a `TBSCertificate`, as defined in [Section
    /// 4.1.2 of RFC 5280].
    ///
    /// [Section 4.1.2 of RFC 5280]: <https://www.rfc-editor.org/info/rfc5280/#section-4.1.2>
    ///
    /// # Errors
    ///
    /// Returns an error if certificate generation fails.
    pub(crate) fn tbs_certificate_der_for_self_signing(self) -> eyre::Result<Vec<u8>> {
        let (verification_key, params) = self.prepare();
        let message_detacher = MessageDetacher::new(verification_key);
        let _certificate = params.self_signed(&message_detacher)?;
        #[expect(clippy::missing_panics_doc, reason = "invariant")]
        Ok(message_detacher
            .into_message()
            .expect("generating a certificate should sign a message"))
    }

    /// Creates a self‐signed certificate with the given signature, and
    /// returns the “PEM” textual encoding of a `Certificate`, as
    /// defined in [Section 4.1.1 of RFC 5280] and [Section 5 of
    /// RFC 7468].
    ///
    /// [Section 4.1.1 of RFC 5280]: <https://www.rfc-editor.org/info/rfc5280/#section-4.1.1>
    /// [Section 5 of RFC 7468]: <https://www.rfc-editor.org/info/rfc7468/#section-5>
    ///
    /// # Errors
    ///
    /// Returns an error if certificate generation fails.
    pub(crate) fn self_signed_certificate_pem(self, signature: Vec<u8>) -> eyre::Result<String> {
        let (verification_key, params) = self.prepare();
        let signature_attacher = SignatureAttacher::new(verification_key, signature);
        let certificate = params.self_signed(&signature_attacher)?;
        Ok(certificate.pem())
    }
}

/// Returns an X.509 public key identifier for the given subject
/// public key, computed from a truncated SHA‐256 hash as specified in
/// [RFC 7093].
///
/// [RFC 7093]: <https://www.rfc-editor.org/info/rfc7093/>
fn rfc7093_truncated_sha256_key_identifier(verification_key: impl SubjectPublicKey) -> [u8; 20] {
    truncated_sha256(&SubjectPublicKeyInfo::new(verification_key).to_der())
}

/// Hashes `message` with SHA‐256, and returns the first 160 bits of
/// the digest.
fn truncated_sha256(message: &[u8]) -> [u8; 20] {
    #[expect(clippy::missing_panics_doc, reason = "invariant")]
    *Sha256::hash(message)
        .as_ref()
        .get(0..20)
        .and_then(|truncated_digest| truncated_digest.as_array())
        .expect("SHA‐256 digest should be 32 bytes")
}
