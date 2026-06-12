# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  writeText,
  runCommand,
  autopen,
  openssl_4_0,
  linkFarmFromDrvs,
}:

{
  signingKey,
}:

let
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
in

linkFarmFromDrvs "autopen-test" [
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
