# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  buildPackages,
  autopen,
  clippy,
  cargo-audit,
  rustsec-advisory-db,
  cargo-deny,
}:

let
  mkCargoCheck =
    cargoCheckCommand: f:
    autopen.overrideAttrs (
      prevAttrs:
      {
        pname = "${prevAttrs.pname}-cargo-${cargoCheckCommand}";

        inherit cargoCheckCommand;
        cargoCheckFlags = [ ];

        doCheck = false;
        dontCargoInstall = true;

        buildPhase = ''
          runHook preBuild

          echoCmd "cargo $cargoCheckCommand flags" "''${cargoCheckFlags[@]}"
          ${buildPackages.rust.envVars.setEnv} \
            cargo "$cargoCheckCommand" "''${cargoCheckFlags[@]}"

          runHook postBuild
        '';

        installPhase = ''
          runHook preInstall

          mkdir -- "$out"

          runHook postInstall
        '';
      }
      // f prevAttrs
    );

  mkCargoBuildCheck =
    cargoCheckCommand: f:
    mkCargoCheck cargoCheckCommand (
      prevAttrs:
      {
        cargoBuildType = "debug";
        cargoCheckType = "debug";

        preBuild = ''
          cargoCheckFlags=(
            -j "$NIX_BUILD_CORES"
            --target "$rustHostPlatformSpec"
            --frozen
            "''${cargoCheckFlags[@]}"
          )
        '';

        inherit (buildPackages.rust.envVars)
          rustHostPlatform
          rustHostPlatformSpec
          ;
      }
      // f prevAttrs
    );
in

{
  cargo-clippy = mkCargoBuildCheck "clippy" (prevAttrs: {
    nativeBuildInputs = prevAttrs.nativeBuildInputs or [ ] ++ [ clippy ];

    cargoCheckFlags = [
      "--all-targets"
      "--no-deps"
      "--"
      "--deny"
      "warnings"
    ];
  });

  cargo-doc = mkCargoBuildCheck "doc" (prevAttrs: {
    cargoCheckFlags = [
      "--no-deps"
      "--document-private-items"
    ];

    dontInstall = false;

    installPhase = ''
      runHook preInstall

      mkdir -p -- "$out/share/doc"
      mv "target/$rustHostPlatform/doc" -- "$out/share/doc/autopen"

      runHook postInstall
    '';

    env = prevAttrs.env or { } // {
      RUSTDOCFLAGS = "--deny warnings";
    };
  });

  cargo-audit = mkCargoCheck "audit" (prevAttrs: {
    nativeBuildInputs = prevAttrs.nativeBuildInputs or [ ] ++ [ cargo-audit ];

    cargoCheckFlags = [
      "--db"
      "${rustsec-advisory-db}"
    ];
  });

  cargo-deny = mkCargoCheck "deny" (prevAttrs: {
    nativeBuildInputs = prevAttrs.nativeBuildInputs or [ ] ++ [ cargo-deny ];

    cargoCheckFlags = [
      "--offline"
      "check"
      "bans"
      "licenses"
      "sources"
      "--deny"
      "warnings"
    ];
  });
}
