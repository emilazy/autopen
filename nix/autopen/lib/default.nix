{ callPackage }:

# TODO: Many of these should be hooks rather than monolithic
# derivation builders.

# TODO: There’s probably issues with cross‐compilation here.

# TODO: Many of these will probably have to handle multiple paths.

{
  internal = callPackage ./internal.nix { };
  signingKey = callPackage ./signing-key.nix { };
  sign = callPackage ./sign.nix { };
  x509 = callPackage ./x509.nix { };
  authenticode = callPackage ./authenticode.nix { };
}
