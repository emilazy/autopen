# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  stdenv,
  writeText,
  runCommand,
  autopen,
  openssl_4_0,
  fwupd-efi,
  sbsigntool,
  linkFarmFromDrvs,
}:

{
  signingKey,
}:

let
  inherit (lib) optionals;

  message = writeText "autopen-test-message" ''
    squeamish ossifrage
  '';

  signature = autopen.lib.sign {
    inherit signingKey;
    message = message;
  };

  check-signature =
    runCommand "autopen-test-check-signature"
      {
        nativeBuildInputs = [
          autopen
        ];
        inherit (signingKey) verificationKey;
        inherit signature message;
        strictDeps = true;
        __structuredAttrs = true;
      }
      ''
        autopen verify \
          --verification-key="$verificationKey" \
          --signature="$signature" \
          -- "$message"
        touch -- "$out"
      '';

  certificate = autopen.lib.x509.mkSelfSignedCertificate {
    name = "autopen-test-certificate";
    inherit signingKey;
    purpose = "code-signing";
    commonName = "Test Organization";
    notBefore = "1970-01-01T00:00:00Z";
    lifetimeDays = 365;
  };

  check-certificate =
    runCommand "autopen-test-check-certificate"
      {
        nativeBuildInputs = [
          # Versions prior to 4.0 suffer from
          # <https://github.com/openssl/openssl/issues/15124>…
          openssl_4_0
        ];
        inherit certificate;
        strictDeps = true;
        __structuredAttrs = true;
      }
      ''
        exec &> >(tee -- "$out")
        openssl asn1parse -in "$certificate" -inform pem -i
        openssl x509 -in "$certificate" -noout -text
        openssl verify \
          -verbose \
          -CAfile "$certificate" \
          -attime "$(date --date=1970-12-31T23:59:59Z +%s)" \
          -check_ss_sig \
          -x509_strict \
          -- "$certificate"
      '';

  fwupd-efi-signed = autopen.lib.authenticode.mkSignedPe {
    name = "autopen-test-fwupd-efi-signed";
    # TODO: Perhaps an abstraction for certificate + signing key would
    # be nice?
    inherit signingKey certificate;
    peFile = "${fwupd-efi}/libexec/fwupd/efi/fwupd${stdenv.hostPlatform.efiArch}.efi";
  };

  check-fwupd-efi-signature =
    runCommand "autopen-test-check-fwupd-efi-signature"
      {
        nativeBuildInputs = [ sbsigntool ];
        inherit certificate;
        signedPeFile = fwupd-efi-signed;
        strictDeps = true;
        __structuredAttrs = true;
      }
      ''
        exec &> >(tee -- "$out")
        sbverify --list -- "$signedPeFile"
        sbverify --cert="$certificate" -- "$signedPeFile"
      '';
in

linkFarmFromDrvs "autopen-test" (
  [
    signingKey
    signingKey.verificationKey

    message
    signature
    check-signature

    certificate
    certificate.tbsCertificate
    certificate.signature
    check-certificate
  ]
  ++ optionals stdenv.hostPlatform.isLinux [
    fwupd-efi-signed
    fwupd-efi-signed.signedAttrs
    fwupd-efi-signed.signature
    check-fwupd-efi-signature
  ]
)
