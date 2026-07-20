{
  lib,
  writeShellApplication,
  autopen,
  socketPath ? "/tmp/autopen/socket",
}:

let
  inherit (lib) escapeShellArg;

  softwareSigningKey = "${./keys/test-rsa3072-pkcs1-sha256-signing-key}";

  remoteSigningKey = autopen.lib.signingKey.remote {
    name = "autopen-test-remote-rsa3072-pkcs1-sha256";
    verificationKey = ./keys/test-rsa3072-pkcs1-sha256-verification-key;
    inherit socketPath;
  };

  server = writeShellApplication {
    name = "autopen-test-remote-run-server";

    runtimeInputs = [ autopen ];

    text = ''
      socket_path=${escapeShellArg socketPath}
      file_ref_path=${escapeShellArg remoteSigningKey.fileRefPath}
      signing_key=${escapeShellArg softwareSigningKey}
      socket_dir=$(dirname -- "$socket_path")

      clean_up() {
        rm -f -- "$socket_path"
      }

      clean_up
      trap clean_up EXIT

      mkdir -p -- "$socket_dir"
      chmod 0755 -- "$socket_dir"

      umask 0000
      set -x
      autopen serve \
        --socket-path "$socket_path" \
        --signing-key-ref "$file_ref_path" "$signing_key" \
        "$@"
    '';

    derivationArgs = {
      strictDeps = true;
      __structuredAttrs = true;
    };
  };

  test = autopen.mkTest {
    signingKey = remoteSigningKey;
  };
in

{
  inherit
    softwareSigningKey
    remoteSigningKey
    server
    test
    ;
}
