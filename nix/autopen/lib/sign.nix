{
  lib,
  autopen,
}:

let
  inherit (lib)
    unsafeGetAttrPos
    ;

  inherit (autopen.lib.internal)
    hideDerivation
    mkAutopenDerivation
    ;
in

{
  signingKey,
  message,
  pos ? unsafeGetAttrPos "message" args,
  meta ? { },
}@args:
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

  inherit pos meta;
})
