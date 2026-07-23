{
  lib,
  autopen,
}:

let
  inherit (lib)
    hashString
    isPath
    unsafeGetAttrPos
    ;

  inherit (autopen.lib.internal)
    hideDerivation
    mkAutopenDerivation
    ;

  # TODO: Explain this.
  fakeDerivation =
    outPath: attrs:
    let
      drv = {
        type = "derivation";

        outputs = [ "out" ];
        out = drv;
        all = [ drv ];
        outputName = "out";

        inherit outPath;
      }
      // attrs;
    in
    drv;
in
{
  # TODO: Document the insecurity of using this with a software key.
  import =
    {
      name,
      path,
      pos ? unsafeGetAttrPos args "name",
      meta ? { },
    }@args:
    let
      signingKey = fakeDerivation "${path}" {
        inherit name verificationKey;
        meta = meta // {
          ${if pos != null then "position" else null} = "${pos.file}:${toString pos.line}";
        };
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

        inherit pos;

        meta = meta // {
          ${if meta ? description then "description" else null} = "${meta.description} (verification key)";
        };
      });
    in
    signingKey;

  remote =
    {
      name,
      socketPath ? "/run/autopen/socket",
      verificationKey,
      pos ? unsafeGetAttrPos args "name",
      meta ? { },
    }@args:
    let
      verificationKeyPath = args.verificationKey;

      verificationKey = fakeDerivation "${verificationKeyPath}" {
        name = "${name}-verification-key";

        meta = meta // {
          ${if meta ? description then "description" else null} = "${meta.description} (verification key)";
          ${if pos != null then "position" else null} = "${pos.file}:${toString pos.line}";
        };
      };

      # TODO: Especially explain this!
      fileRefPath = builtins.path {
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

      inherit pos meta;
    };
}
