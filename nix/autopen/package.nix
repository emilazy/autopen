{
  lib,
  stdenv,
  rustPlatform,
  capnproto,
  newScope,
  testers,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "autopen";
  version = "0.1.0";

  src =
    let
      cargoToml = lib.importTOML ../../Cargo.toml;
    in
    lib.fileset.toSource {
      root = ../../.;
      fileset = lib.fileset.unions (map (subPath: ../../. + subPath) cargoToml.package.include);
    };

  cargoLock = {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "capnp-0.26.2" = "sha256-K7Loo9KhZ0wUY/NMrgu1WkftA4MBF2m43ZzCwVr0YAk=";
    };
  };

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
      scope = lib.makeScope newScope (_self: {
        inherit autopen;
      });
    in
    {
      lib = scope.callPackage ./lib { };

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
      // lib.optionalAttrs (!stdenv.hostPlatform.isDarwin) {
        nixos = testers.runNixOSTest {
          imports = [ ./tests/nixos.nix ];
          _module.args = { inherit autopen; };
        };
      };
    };

  meta = {
    description = "Cryptographic signing tool with an object‐capability interface";
    homepage = "https://github.com/emilazy/autopen";
    license = lib.licenses.blueOak100;
    sourceProvenance = [ lib.sourceTypes.fromSource ];
    maintainers = [ lib.maintainers.emily ];
    mainProgram = "autopen";
    platforms = lib.platforms.unix;
  };
})
