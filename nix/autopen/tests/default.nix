{
  lib,
  stdenv,
  writeText,
  runCommand,
  autopen,
  openssl_4_0,
  fwupd-efi,
  pesign,
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
        openssl asn1parse -in "$certificate" -inform DER -i
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
        nativeBuildInputs = [
          pesign
          openssl_4_0
        ];
        inherit certificate;
        signedPeFile = fwupd-efi-signed;
        strictDeps = true;
        __structuredAttrs = true;
      }
      ''
        exec &> >(tee -- "$out")
        pesign --export-signature=signature.p7m --in="$signedPeFile"
        openssl asn1parse -inform DER -i -in signature.p7m
        pesign --list-signatures --in="$signedPeFile"
        pesigcheck --no-system-db=0 --certfile="$certificate" --in="$signedPeFile"
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
