# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  stdenvNoCC,
  buildPackages,
  autopen,
}:

# TODO: Many of these should be hooks rather than monolithic
# derivation builders.

# TODO: There’s probably issues with cross‐compilation here.

# TODO: Many of these will probably have to handle multiple paths.

let
  inherit (builtins)
    path
    ;

  inherit (lib)
    concatMap
    concatMapStringsSep
    extendMkDerivation
    getLib
    hashString
    head
    isAttrs
    isDerivation
    isPath
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

  mkSystemdSbsignDerivation = mkCliDerivationBuilder {
    package = getLib buildPackages.systemd;
    exe = "${getLib buildPackages.systemd}/lib/systemd/systemd-sbsign";
    argsAttrName = "systemdSbsignArgs";
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

    remote =
      {
        name,
        socketPath ? "/run/autopen/socket",
        verificationKey,
      }@args:
      let
        verificationKeyPath = args.verificationKey;

        verificationKey = pathDerivation verificationKeyPath {
          name = "${name}-verification-key";
        };

        # TODO: Especially explain this!
        fileRefPath = path {
          path = ./.;
          name = "${name}-key-handle-${hashString "sha256" "${verificationKey}"}";
          filter = _: _: false;
        };
      in
      assert isPath verificationKeyPath;
      mkAutopenDerivation {
        name = "${name}-signing-key";

        autopenArgs = [
          "signing-key"
          "remote"
          "create"
          {
            inherit socketPath fileRefPath verificationKey;
            output = placeholder "out";
          }
        ];

        allowedRequisites = [
          "out"
          fileRefPath
        ];

        passthru = {
          inherit socketPath fileRefPath verificationKey;
        };
      };
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

  x509 = {
    mkTbsCertificateForSelfSigning =
      { name, ... }@args:
      mkAutopenDerivation (finalAttrs: {
        inherit name;

        autopenArgs = [
          "x509"
          "create-tbs-certificate"
          finalAttrs.passthru.certificateParams
          { output = placeholder "out"; }
        ];

        passthru = {
          inherit (finalAttrs.passthru.certificateParams) verificationKey;
          certificateParams = removeAttrs args [ "name" ];
        };
      });

    attachSignature =
      {
        name,
        signature,
      }:
      let
        tbsCertificate = signature.message;
      in
      mkAutopenDerivation {
        inherit name;

        autopenArgs = [
          "x509"
          "create-certificate"
          tbsCertificate.certificateParams
          {
            inherit signature;
            output = placeholder "out";
          }
        ];

        passthru = {
          inherit (tbsCertificate) verificationKey certificateParams;
          inherit tbsCertificate signature;
        };
      };

    mkSelfSignedCertificate =
      { name, signingKey, ... }@args:
      x509.attachSignature {
        name = "${name}.crt";
        signature = sign {
          inherit signingKey;
          message = x509.mkTbsCertificateForSelfSigning (
            removeAttrs args [
              "name"
              "signingKey"
            ]
            // {
              name = "${name}.tbs-certificate.der";
              inherit (signingKey) verificationKey;
            }
          );
        };
      };
  };

  authenticode = {
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
      authenticode.attachSignature {
        name = "${name}.${peExtension}";
        signature = sign {
          inherit signingKey;
          message = authenticode.mkSignedAttrsForPe (
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
  };
in
{
  inherit
    sign
    signingKey
    x509
    authenticode
    ;
}
