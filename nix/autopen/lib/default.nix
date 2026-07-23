{ callPackage }:

# TODO: There’s probably issues with cross‐compilation here.

{
  internal = callPackage ./internal.nix { };
  signingKey = callPackage ./signing-key.nix { };
  sign = callPackage ./sign.nix { };
  x509 = callPackage ./x509.nix { };
  authenticode = callPackage ./authenticode.nix { };
}
