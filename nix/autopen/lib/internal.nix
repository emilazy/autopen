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
    substring
    toLower
    toUpper
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
      attrPrefix,
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
            runHook "pre$hookName"

            echoCmd "$exeName flags" "''${exeFlags[@]}"
            "$exe" "''${exeFlags[@]}"

            runHook "post$hookName"
          '';

          inherit exe;

          exeName = baseNameOf exe;

          exeFlags = concatMap (
            component:
            if isAttrs component && !isDerivation component then
              toCommandLine optionFormat component
            else
              [ (mkValueStringDefault { } component) ]
          ) args."${attrPrefix}Args";

          hookName = toUpper (substring 0 1 attrPrefix) + substring 1 (-1) attrPrefix;

          strictDeps = true;

          __structuredAttrs = true;

          # TODO: `meta`.
        };
    };

  mkAutopenDerivation = mkCliDerivationBuilder {
    package = autopen;
    exe = "autopen";
    attrPrefix = "autopen";
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
