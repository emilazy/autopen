{
  lib,
  buildPackages,
  autopen,
}:

let
  inherit (lib)
    concatMapStringsSep
    escapeURL
    getLib
    head
    match
    splitString
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
    attrPrefix = "systemdSbsign";
  };

  escapeURLPath = path: concatMapStringsSep "/" escapeURL (splitString "/" path);
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
        finalAttrs.certificateArgs
        {
          prepareOfflineSigning = true;
          output = placeholder "out";
        }
        peFile
      ];

      # `systemd-sbsign(1)` expects a PEM‐encoded certificate, but
      # autopen produces DER-encoded certificates. We explicitly
      # use the default OpenSSL provider, which takes `file://`
      # URLs and accepts both encodings.
      certificateArgs = {
        certificateSource = "provider:default";
        certificate = "file://${escapeURLPath "${certificate}"}";
      };

      certificateNotBefore = certificate.certificateParams.notBefore;

      preSystemdSbsign = ''
        export SOURCE_DATE_EPOCH="$(date --date="$certificateNotBefore" +%s)"
      '';

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
        signedAttrs.certificateArgs
        {
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
