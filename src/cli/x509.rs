// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! The `autopen x509` subcommand.

use camino::Utf8PathBuf;
use clap::builder::TypedValueParser;
use color_eyre::eyre::{self, WrapErr as _};
use tokio::fs;

use crate::{
    cli::Subcommand,
    local::Bootstrap,
    verification_key::VerificationKey,
    x509::{CertificateBuilder, CertificateValidityPeriod, KeyPurpose},
};

/// Commands for X.509 certificates.
#[cfg_attr(
    // Work around <https://github.com/rust-lang/rust-clippy/issues/16934>.
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "subcommands are documented by their respective types"
    )
)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    CreateTbsCertificate(CreateTbsCertificate),
    CreateCertificate(CreateCertificate),
}

impl Subcommand for Command {
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        match self {
            Self::CreateTbsCertificate(cmd) => cmd.run(local).await,
            Self::CreateCertificate(cmd) => cmd.run(local).await,
        }
    }
}

// TODO: The duplication with `crate::x509::CertificateBuilder` is
// pretty annoying.
/// Command‐line options for X.509 certificate generation parameters.
#[derive(Debug, clap::Args)]
struct CertificateOptions {
    /// The verification key to use as the certificate subject’s
    /// public key.
    #[arg(long, value_name = "PATH")]
    verification_key: Utf8PathBuf,
    /// The purpose for which the public key is certified for use.
    #[arg(long, value_enum)]
    purpose: KeyPurpose,
    /// The certificate subject’s common name.
    #[arg(long)]
    common_name: String,
    /// The start of the certificate’s validity period.
    #[arg(long, value_name = "DATE_TIME", value_parser = utc_date_time_parser())]
    not_before: time::UtcDateTime,
    /// The duration of the certificate’s validity period in days.
    #[arg(long = "lifetime-days", value_name = "DAYS", value_parser = days_parser())]
    lifetime: time::Duration,
}

/// Returns a parser for the Internet date/time format from [Section
/// 5.6 of RFC 3339].
///
/// [Section 5.6 of RFC 3339]: <https://www.rfc-editor.org/info/rfc3339/#section-5.6>
fn utc_date_time_parser() -> impl TypedValueParser<Value = time::UtcDateTime> {
    use time::format_description::well_known::Rfc3339;
    |arg: &str| time::UtcDateTime::parse(arg, &Rfc3339)
}

/// Returns a parser for integer numbers of days between 1 and
/// 365 (inclusive).
fn days_parser() -> impl TypedValueParser<Value = time::Duration> {
    clap::value_parser!(i64)
        .range(1..=365)
        .map(time::Duration::days)
}

impl CertificateOptions {
    /// Loads the provided verification key and returns a certificate
    /// builder for the given options.
    ///
    /// # Errors
    ///
    /// Forwards errors on from [`Bootstrap::load()`].
    async fn into_certificate_builder(
        self,
        local: &Bootstrap,
    ) -> eyre::Result<CertificateBuilder<VerificationKey>> {
        let verification_key = local.load(&self.verification_key).await?;
        Ok(CertificateBuilder {
            verification_key,
            purpose: self.purpose,
            common_name: self.common_name,
            validity_period: CertificateValidityPeriod::new(self.not_before, self.lifetime)?,
        })
    }
}

/// Create an X.509 `TBSCertificate` for self‐signing.
///
/// You can use `autopen sign` to sign the resulting `TBSCertificate`
/// and `autopen x509 create-certificate` to create a certificate with
/// the resulting signature.
#[derive(Debug, clap::Args)]
pub(crate) struct CreateTbsCertificate {
    /// The file to write the DER‐encoded `TBSCertificate` to.
    #[arg(long, value_name = "PATH")]
    output: Utf8PathBuf,
    /// The certificate parameters.
    #[command(flatten, next_help_heading = "Certificate options")]
    certificate_options: CertificateOptions,
}

impl Subcommand for CreateTbsCertificate {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let builder = self
            .certificate_options
            .into_certificate_builder(&local)
            .await?;
        let tbs_certificate_der = builder
            .tbs_certificate_der_for_self_signing()
            .wrap_err("Failed to create TBSCertificate")?;
        fs::write(&self.output, tbs_certificate_der)
            .await
            .wrap_err_with(|| format!("Failed to write output file {}", self.output))?;
        Ok(())
    }
}

/// Create a self‐signed X.509 `Certificate` with a provided signature.
///
/// You can use `autopen x509 create-tbs-certificate` to create an X.509
/// `TBSCertificate` and `autopen sign` to produce a corresponding
/// signature to use with this command. The certificate options of the
/// `TBSCertificate` must match the options provided to this command.
#[derive(Debug, clap::Args)]
pub(crate) struct CreateCertificate {
    /// The certificate’s signature.
    ///
    /// This must match the certificate subject’s public key.
    #[arg(long, value_name = "PATH")]
    signature: Utf8PathBuf,
    /// The file to write the DER‐encoded `Certificate` to.
    #[arg(long, value_name = "PATH")]
    output: Utf8PathBuf,
    /// The certificate parameters.
    #[command(flatten, next_help_heading = "Certificate options")]
    certificate_options: CertificateOptions,
}

impl Subcommand for CreateCertificate {
    #[tracing::instrument(level = tracing::Level::DEBUG)]
    async fn run(self, local: Bootstrap) -> eyre::Result<()> {
        let builder = self
            .certificate_options
            .into_certificate_builder(&local)
            .await?;
        let signature = fs::read(&self.signature)
            .await
            .wrap_err_with(|| format!("Failed to read signature file {}", self.signature))?;
        let certificate_der = builder
            .self_signed_certificate_der(signature)
            .wrap_err("Failed to create certificate")?;
        fs::write(&self.output, certificate_der)
            .await
            .wrap_err_with(|| format!("Failed to write output file {}", self.output))?;
        Ok(())
    }
}
