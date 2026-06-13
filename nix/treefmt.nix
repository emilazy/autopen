# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  config,
  pkgs,
  ...
}:

let
  inherit (lib)
    attrValues
    concatMap
    getExe
    mkAfter
    ;
in

{
  settings = {
    on-unmatched = "fatal";

    excludes = [
      ".editorconfig"
      "*.capnp"
      "*.license"
      "LICENSES/*.txt"
      "nix/autopen/tests/keys/*"
    ];

    formatter = {
      editorconfig-checker = {
        command = getExe pkgs.editorconfig-checker;
        includes =
          let
            # Cover every file we would otherwise format rather than
            # including every file directly, to ensure that the
            # `on-unmatched` setting fires correctly when we don’t have
            # an explicit formatter configured.
            allFormatters = config.settings.formatter;
            otherFormatters = removeAttrs allFormatters [ "editorconfig-checker" ];
          in
          concatMap (formatter: formatter.includes) (attrValues otherFormatters);
      };

      rumdl-check = {
        options = mkAfter [
          "--config"
          "${(pkgs.formats.toml { }).generate ".rumdl.toml" {
            MD013 = {
              reflow = true;
              reflow-mode = "normalize";
            };
          }}"
        ];
      };
    };
  };

  programs = {
    nixfmt = {
      enable = true;
    };

    rustfmt = {
      enable = true;
    };

    rumdl-check = {
      enable = true;
    };

    shfmt = {
      enable = true;
    };

    shellcheck = {
      enable = true;
      extra-checks = [ "all" ];
    };

    taplo = {
      enable = true;

      settings = {
        formatting = {
          align_comments = false;
          indent_string = "    ";
          reorder_keys = true;
          reorder_arrays = true;
          reorder_inline_tables = true;
        };

        rule = [
          {
            include = [ "**/Cargo.toml" ];
            keys = [ "package" ];
            formatting = {
              reorder_keys = false;
            };
          }

          {
            include = [ "**/Cargo.toml" ];
            keys = [
              "dependencies"
              "dev-dependencies"
              "build-dependencies"
              "target.*.dependencies"
              "target.*.dev-dependencies"
              "target.*.build-dependencies"
            ];
            formatting = {
              reorder_inline_tables = false;
            };
          }

          {
            include = [ "**/clippy.toml" ];
            formatting = {
              reorder_arrays = false;
            };
          }
        ];
      };
    };
  };
}
