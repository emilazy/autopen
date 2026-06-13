# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  writeText,
  runCommand,
  autopen,
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
in

linkFarmFromDrvs "autopen-test" [
  signingKey
  signingKey.verificationKey

  message
  signature
  check-signature
]
