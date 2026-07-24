{
  lib,
  stdenvNoCC,
  autopen,
}:

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

  inherit (autopen.lib.internal)
    mkCliDerivationBuilder
    ;

  splitCamelCase = splitStringBy (_prev: curr: match "[A-Z]" curr != null) true;

  toKebabCase = camelCase: concatMapStringsSep "-" toLower (splitCamelCase camelCase);

  optionFormat = optionName: {
    option = "--${toKebabCase optionName}";
    sep = "=";
    explicitBool = false;
  };
in
{
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
}
