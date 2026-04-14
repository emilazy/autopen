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
    ;
in

{
  settings = {
    on-unmatched = "fatal";

    excludes = [
      ".editorconfig"
      "*.license"
      "LICENSES/*.txt"
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
    };
  };

  programs = {
    nixfmt = {
      enable = true;
    };

    shfmt = {
      enable = true;
    };

    shellcheck = {
      enable = true;
      extra-checks = [ "all" ];
    };
  };
}
