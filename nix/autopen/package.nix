# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  stdenv,
  rustPlatform,
  capnproto,
  newScope,
  testers,
}:

let
  inherit (lib)
    fileset
    importTOML
    licenses
    maintainers
    makeScope
    optionalAttrs
    platforms
    sourceTypes
    ;

  cargoToml = importTOML ../../Cargo.toml;

  src = fileset.toSource {
    root = ../../.;
    fileset = fileset.unions (map (subPath: ../../. + subPath) cargoToml.package.include);
  };

  cargoLock = {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "capnp-0.25.5" = "sha256-L8mVQIY3trr5ORFVRdOD83Uurt2f5/qaLZJuwr8a7LI=";
    };
  };
in

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "autopen";
  version = "0.1.0";

  inherit src cargoLock;

  nativeBuildInputs = [
    capnproto
  ];

  useNextest = true;

  cargoTestFlags = [ "--max-fail=all" ];

  strictDeps = true;

  __structuredAttrs = true;

  passthru =
    let
      # TODO: Can this work with cross‐compilation?
      autopen = finalAttrs.finalPackage;
      scope = makeScope newScope (_self: {
        inherit autopen;
      });
    in
    {
      lib = scope.callPackage ./lib.nix { };

      mkTest = scope.callPackage ./tests { };

      test-remote = scope.callPackage ./tests/remote.nix { };

      tests = {
        software-key = autopen.mkTest {
          signingKey = autopen.lib.signingKey.import {
            name = "autopen-test-rsa3072-pkcs1-sha256";
            path = ./tests/keys/test-rsa3072-pkcs1-sha256-signing-key;
          };
        };
      }
      # TODO: There seems to be some Nixpkgs regression that makes
      # starting the VM test hang on macOS.
      // optionalAttrs (!stdenv.hostPlatform.isDarwin) {
        nixos = testers.runNixOSTest {
          imports = [ ./tests/nixos.nix ];
          _module.args = { inherit autopen; };
        };
      };
    };

  meta = {
    description = "Cryptographic signing tool with an object‐capability interface";
    homepage = "https://github.com/emilazy/autopen";
    license = licenses.blueOak100;
    sourceProvenance = [ sourceTypes.fromSource ];
    maintainers = [ maintainers.emily ];
    mainProgram = "autopen";
    platforms = platforms.unix;
  };
})
