# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  stdenvNoCC,
  autopen,
}:

# TODO: Many of these should be hooks rather than monolithic
# derivation builders.

# TODO: There’s probably issues with cross‐compilation here.

# TODO: Many of these will probably have to handle multiple paths.

let
  inherit (lib)
    concatMap
    concatMapStringsSep
    extendMkDerivation
    isAttrs
    isDerivation
    match
    splitStringBy
    toLower
    ;

  inherit (lib.generators)
    mkValueStringDefault
    ;

  inherit (lib.cli)
    toCommandLine
    ;

  splitCamelCase = splitStringBy (_prev: curr: match "[A-Z]" curr != null) true;

  toKebabCase = camelCase: concatMapStringsSep "-" toLower (splitCamelCase camelCase);

  optionFormat = optionName: {
    option = "--${toKebabCase optionName}";
    sep = "=";
    explicitBool = false;
  };

  mkCliDerivationBuilder =
    {
      package,
      exe,
      argsAttrName,
    }:
    extendMkDerivation {
      constructDrv = stdenvNoCC.mkDerivation;

      extendDrvArgs =
        finalAttrs:
        {
          nativeBuildInputs ? [ ],
          ...
        }@args:
        {
          nativeBuildInputs = [ package ] ++ nativeBuildInputs;

          buildCommand = ''
            echoCmd "$exeName flags" "''${exeFlags[@]}"
            "$exe" "''${exeFlags[@]}"
          '';

          inherit exe;

          exeName = baseNameOf exe;

          exeFlags = concatMap (
            component:
            if isAttrs component && !isDerivation component then
              toCommandLine optionFormat component
            else
              [ (mkValueStringDefault { } component) ]
          ) args.${argsAttrName};

          strictDeps = true;

          __structuredAttrs = true;

          # TODO: `meta`.
        };
    };

  mkAutopenDerivation = mkCliDerivationBuilder {
    package = autopen;
    exe = "autopen";
    argsAttrName = "autopenArgs";
  };

  # TODO: Explain this.
  pathDerivation =
    path: attrs:
    let
      drv = {
        type = "derivation";

        outputs = [ "out" ];
        out = drv;
        all = [ drv ];
        outputName = "out";

        outPath = "${path}";
      }
      // attrs;
    in
    drv;

  # TODO: Explain this, too.
  hideDerivation =
    drv:
    assert isDerivation drv && drv.outputs == [ "out" ];
    let
      hiddenDrv = {
        inherit (drv)
          type
          name
          system
          outPath
          drvPath
          outputs
          outputName
          strictDeps
          __structuredAttrs
          passthru
          meta
          ;

        out = hiddenDrv;
        all = [ hiddenDrv ];
      }
      // drv.passthru;
    in
    hiddenDrv;

  signingKey = {
    # TODO: Document the insecurity of using this with a software key.
    import =
      { name, path }:
      let
        signingKey = pathDerivation path {
          inherit name;
          inherit verificationKey;
        };
        verificationKey = hideDerivation (mkAutopenDerivation {
          name = "${name}-verification-key";
          autopenArgs = [
            "signing-key"
            "get-verification-key"
            {
              inherit signingKey;
              output = placeholder "out";
            }
          ];
        });
      in
      signingKey;
  };

  sign =
    {
      signingKey,
      message,
    }:
    hideDerivation (mkAutopenDerivation {
      name = "${message.name}.sig";

      inherit signingKey message;

      autopenArgs = [
        "sign"
        {
          inherit signingKey;
          output = placeholder "out";
        }
        message
      ];

      # Keys shouldn’t propagate to outputs.
      allowedRequisites = [ "out" ];

      passthru = {
        # TODO: This is technically a little strange in terms of the
        # cryptographic semantics, but it’s convenient.
        inherit message;
      };
    });
in
{
  inherit
    sign
    signingKey
    ;
}
