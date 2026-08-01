{
  lib,
  stdenvNoCC,
  writeText,
  autopen,
  fwupd-efi,
  linkFarm,
  runCommand,
  openssl_4_0,
  pesign,
}:

{
  signingKey,
}:

let
  message = writeText "autopen-test-message" ''
    squeamish ossifrage
  '';

  signature = autopen.lib.sign {
    inherit signingKey message;
  };

  certificate = autopen.lib.x509.mkSelfSignedCertificate {
    name = "autopen-test-certificate";

    inherit signingKey;

    certificateParams = {
      purpose = "code-signing";
      commonName = "INSECURE TEST CERTIFICATE, DO NOT TRUST";
      notBefore = "1970-01-01T00:00:00Z";
      lifetimeDays = 1;
    };
  };

  fwupd-efi-signed = autopen.lib.authenticode.mkSignedPe {
    pname = "autopen-test-${fwupd-efi.pname}";
    inherit (fwupd-efi) version;
    # TODO: Perhaps an abstraction for certificate + signing key would
    # be nice?
    inherit signingKey certificate;
    peFile = "${fwupd-efi}/libexec/fwupd/efi/fwupd${stdenvNoCC.hostPlatform.efiArch}.efi";
  };
in
linkFarm "autopen-test" (
  {
    inherit signingKey;
    inherit (signingKey) verificationKey;

    inherit message signature;

    signature-check =
      runCommand "autopen-test-signature-check"
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

    inherit certificate;

    certificate-check =
      runCommand "autopen-test-certificate-check"
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
            -attime "$(date --date=1970-01-01T23:59:59Z +%s)" \
            -check_ss_sig \
            -x509_strict \
            -- "$certificate"
        '';
  }
  // lib.optionalAttrs stdenvNoCC.hostPlatform.isLinux {
    inherit fwupd-efi-signed;
    inherit (fwupd-efi-signed) signedAttrs signature;

    fwupd-efi-signed-check =
      runCommand "autopen-test-fwupd-efi-signed-check"
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
  }
)
