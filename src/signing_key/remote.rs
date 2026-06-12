// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Remote signing keys.

use std::{
    fmt::{self, Debug},
    rc::Rc,
};

use capnp::{capability::FromClientHook as _, message};
use tracing::{debug, record_all};

use crate::{
    autopen_capnp::{local::remote_ref, signer, signing_key::remote},
    local::{Serialize, restorer},
    verification_key::{VerificationKey, Verifier as _, VerifyError},
};

/// A signing key accessed through a server.
pub(crate) struct SigningKey {
    /// A client for the remote signing key.
    remote_signer: signer::Client,
    /// A persistent reference to the remote signing key.
    remote_ref: message::TypedBuilder<remote_ref::Owned>,
    /// The verification key corresponding to the remote signing key.
    verification_key: VerificationKey,
}

impl SigningKey {
    /// Creates a remote signing key from a restorer, remote reference,
    /// and verification key.
    ///
    /// The reference will be restored to handle signing requests.
    ///
    /// # Errors
    ///
    /// Returns an error if the remote reference structure is invalid.
    pub(crate) fn new(
        restorer: &restorer::Client,
        remote_ref_reader: remote_ref::Reader<'_>,
        verification_key: VerificationKey,
    ) -> capnp::Result<Self> {
        let mut request = restorer.restore_request();
        request.get().set_sturdy_ref(remote_ref_reader)?;
        let mut remote_ref = message::TypedBuilder::new_default();
        remote_ref.set_root(remote_ref_reader)?;
        Ok(Self {
            remote_signer: request.send().pipeline.get_cap().as_cap().cast_to(),
            remote_ref,
            verification_key,
        })
    }
}

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
        let params = params.get()?;
        let message = params.get_message()?;
        record_all!(tracing::Span::current(), params.message = message);

        let mut request = self.remote_signer.sign_request();
        request.set(params)?;
        let response = request.send().promise.await?;
        let response = response.get()?;
        let signature = response.get_signature()?;

        // We verify the returned signature, mostly because it’s cheap
        // and can help to catch misconfiguration issues. (It also adds
        // some determinism, but avoiding determinism issues caused by
        // deliberate privileged configuration is not a high priority.)
        self.verification_key
            .verify(message, signature)
            .map_err(|VerifyError| {
                capnp::Error::failed("Remote signer returned invalid signature".to_owned())
            })?;
        let mut results = results.get();
        results.set_signature(signature);
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
        self.verification_key
            .build_capnp(results.reborrow().init_verification_key())?;
        debug!(results = ?results.into_reader());
        Ok(())
    }
}

impl Serialize for SigningKey {
    type Owned = remote::Owned;

    fn read_capnp(restorer: &restorer::Client, reader: remote::Reader<'_>) -> capnp::Result<Self> {
        Self::new(
            restorer,
            reader.get_remote_ref()?,
            VerificationKey::read_capnp(restorer, reader.get_verification_key()?)?,
        )
    }

    fn build_capnp(&self, mut builder: remote::Builder<'_>) -> capnp::Result<()> {
        builder
            .reborrow()
            .set_remote_ref(self.remote_ref.get_root_as_reader()?)?;
        self.verification_key
            .build_capnp(builder.init_verification_key())?;
        Ok(())
    }
}

impl Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigningKey")
            .field(
                "remote_ref",
                &self
                    .remote_ref
                    .get_root_as_reader()
                    .expect("`remote_ref` should be valid"),
            )
            .field("verification_key", &self.verification_key)
            .finish_non_exhaustive()
    }
}
