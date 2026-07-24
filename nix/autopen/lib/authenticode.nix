{
  lib,
  buildPackages,
  autopen,
}:

let
  inherit (lib)
    getLib
    head
    match
    ;

  inherit (autopen.lib)
    sign
    ;

  inherit (autopen.lib.internal)
    mkCliDerivationBuilder
    ;

  inherit (autopen.lib.authenticode)
    attachSignature
    mkSignedAttrsForPe
    ;

  mkSystemdSbsignDerivation = mkCliDerivationBuilder {
    package = getLib buildPackages.systemd;
    exe = "${getLib buildPackages.systemd}/lib/systemd/systemd-sbsign";
    argsAttrName = "systemdSbsignArgs";
  };
in
{
  mkSignedAttrsForPe =
    {
      name,
      certificate,
      peFile,
    }:
    mkSystemdSbsignDerivation (finalAttrs: {
      inherit name;

      systemdSbsignArgs = [
        "sign"
        {
          inherit certificate;
          prepareOfflineSigning = true;
          output = placeholder "out";
        }
        peFile
      ];

      passthru = {
        inherit certificate peFile;
      };
    });

  attachSignature =
    {
      name,
      signature,
    }:
    let
      signedAttrs = signature.message;
    in
    mkSystemdSbsignDerivation {
      inherit name;

      systemdSbsignArgs = [
        "sign"
        {
          inherit (signedAttrs) certificate;
          signedData = signedAttrs;
          signedDataSignature = signature;
          output = placeholder "out";
        }
        signedAttrs.peFile
      ];

      passthru = {
        inherit (signedAttrs) certificate peFile;
        inherit signedAttrs signature;
      };
    };

  mkSignedPe =
    {
      name,
      signingKey,
      peFile,
      ...
    }@args:
    let
      peExtension = head (match ".+\\.([^.]+)" (baseNameOf peFile));
    in
    attachSignature {
      name = "${name}.${peExtension}";
      signature = sign {
        inherit signingKey;
        message = mkSignedAttrsForPe (
          removeAttrs args [
            "name"
            "signingKey"
          ]
          // {
            name = "${name}.signed-attrs.der";
          }
        );
      };
    };
}
