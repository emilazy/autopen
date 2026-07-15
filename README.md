<!--
SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>

SPDX-License-Identifier: BlueOak-1.0.0
-->

# autopen

A cryptographic signing service with an object‐capability interface.

This is currently a basic prototype supporting software keys and signing over
Unix sockets, modelling durable references to signing capabilities using the
identities of accessible files; it is not yet ready for production and you
shouldn’t use it for anything.

Forthcoming are support for signing over the network, better signature
algorithms, hardware‐backed keys, and transparency logs.

There is a
[detailed explanation of the design from a Nix perspective](docs/nix-perspective.md),
covering its suitability for integrating signing into Nix builds while
maintaining essential reproducibility properties.

## Building

Building autopen requires Rust 1.95 or later, and the
[Cap’n Proto](https://capnproto.org/) `capnp(1)` tool. This repository includes
a Nix flake with an `autopen` package.

## Usage

You can create a software signing key, obtain its corresponding verification
key, sign a message, and then verify it:

```console
$ autopen signing-key software rsa3072-pkcs1-sha256 generate \
    --output test-signing-key
$ autopen signing-key get-verification-key \
    --signing-key test-signing-key \
    --output test-verification-key
$ printf 'squeamish ossifrage\n' > test-message
$ autopen sign \
    --signing-key test-signing-key \
    --output test-signature \
    test-message
$ autopen verify \
    --verification-key test-verification-key \
    --signature test-signature \
    test-message
```

You can also provide access to signing keys over a Unix socket without exposing
the key material to clients:

```console
$ mkdir test-key-reference
$ autopen signing-key remote create \
    --socket-path test-socket \
    --file-ref-path test-key-reference \
    --verification-key test-verification-key \
    --output test-remote-key
$ autopen serve \
    --socket-path test-socket \
    --signing-key-ref test-key-reference test-signing-key
```

Then, in another shell:

```console
$ printf 'hapax legomenon\n' > test-message-2
$ autopen sign \
    --signing-key test-remote-key \
    --output test-signature-2 \
    test-message-2
$ autopen verify \
    --verification-key test-verification-key \
    --signature test-signature-2 \
    test-message-2
```

To use the key, clients must be able to connect to the socket path encoded in
the remote key, and open the file reference path to pass as a file description
over that socket.

See `autopen --help` for more detail, including utility commands to produce
self‐signed X.509 code signing certificates.

### Nix library

There is a Nix library for integrating autopen signing, accessible as
`autopen.lib` and implemented in `nix/lib.nix`.

There is currently no documentation, but the following tests may be helpful for
understanding the API:

* `autopen.mkTest` (`nix/autopen/tests/default.nix`) takes a caller‐specified
  signing key and produces an X.509 code signing certificate for it. On Linux,
  the certificate is then used to sign the fwupd UEFI executable from Nixpkgs.
* `autopen.tests.softwareKey` instantiates `autopen.test` with a test software
  key.
* `autopen.remoteKeyTest` (`nix/autopen/tests/remote.nix`) contains
  `autopen.remoteKeyTest.server`, which starts a signing server on
  `/tmp/autopen/socket` when run, and `autopen.remoteKeyTest.test`, which
  instantiates `autopen.test` with the corresponding remote signing key. After
  starting the server, you can then build the test with
  `--extra-sandbox-paths /tmp/autopen`.
* `autopen.tests.nixos` (`nix/autopen/tests/nixos.nix`) is an end‐to‐end NixOS
  VM test that runs autopen as a sandboxed systemd service and automates the
  remote test build.

## Funding

This project is funded through
[NGI Fediversity Fund](https://nlnet.nl/fediversity/), a fund established by
[NLnet](https://nlnet.nl/) with financial support from the European Commission’s
[Next Generation Internet](https://ngi.eu/) programme. Learn more at the
[NLnet project page](https://nlnet.nl/project/NixOS-verifiedboot/).

## Licence

autopen is available under the
[Blue Oak Model License 1.0.0](LICENSES/BlueOak-1.0.0.txt). The repository is
compliant with the [REUSE](https://reuse.software/) specification.
