# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      treefmt-nix,
    }:
    let
      inherit (nixpkgs.lib)
        attrValues
        genAttrs
        ;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      eachSystem = f: genAttrs systems (system: f system nixpkgs.legacyPackages.${system});

      treefmtEval = eachSystem (_system: pkgs: treefmt-nix.lib.evalModule pkgs ./nix/treefmt.nix);

      devChecks = eachSystem (
        system: pkgs: {
          reuse = pkgs.runCommand "reuse-check" { nativeBuildInputs = [ pkgs.reuse ]; } ''
            cd ${self}
            reuse lint
            touch $out
          '';

          treefmt = treefmtEval.${system}.config.build.check self;
        }
      );
    in
    {
      checks = eachSystem (system: _pkgs: devChecks.${system});

      devShells = eachSystem (
        system: pkgs: {
          default = pkgs.mkShell {
            inputsFrom = attrValues devChecks.${system};
            packages = attrValues treefmtEval.${system}.config.build.programs;
          };
        }
      );

      formatter = eachSystem (system: _pkgs: treefmtEval.${system}.wrapper);
    };
}
