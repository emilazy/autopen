{
  lib,
  autopen,
}:

let
  inherit (builtins)
    path
    ;

  inherit (lib)
    hashString
    isPath
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
    { name, path }:
    let
      signingKey = fakeDerivation "${path}" {
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

      verificationKey = fakeDerivation "${verificationKeyPath}" {
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
}
