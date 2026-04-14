# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  rustPlatform,
}:

let
  inherit (lib)
    fileset
    importTOML
    licenses
    maintainers
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
  };
in

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "autopen";
  version = "0.1.0";

  inherit src cargoLock;

  useNextest = true;

  cargoTestFlags = [ "--max-fail=all" ];

  strictDeps = true;

  __structuredAttrs = true;

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
