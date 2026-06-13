# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

{
  lib,
  config,
  autopen ? config.node.pkgs.autopen,
  ...
}:

let
  inherit (lib)
    fakeHash
    unsafeDiscardOutputDependency
    ;

  test-remote = autopen.test-remote.override {
    socketPath = "/run/autopen/socket";
  };

  inherit (test-remote)
    softwareSigningKey
    remoteSigningKey
    test
    ;

  testDrvPath = unsafeDiscardOutputDependency test.drvPath;

  # We need the build dependency closure of the test derivation to be
  # present in the test node’s Nix store so we can try to build it, but
  # if we included `test.drvPath` in `virtualisation.additionalPaths`,
  # it would try to build all the derivations on the host first to
  # include their outputs, likely failing (as we can’t expect to build
  # derivations requiring access to the autopen socket on the host).
  #
  # `unsafeDiscardOutputDependency` is meant to fix this, but is broken
  # with `exportReferencesGraph`; see
  # <https://github.com/NixOS/nix/issues/7330>. Instead, we do a
  # horrific hack: get a derivation that depends on the right things to
  # list the entire visible contents of the store within its sandbox,
  # in stubbed‐out `nix-store --dump-db` format. Then we can register
  # the paths and go about our business as usual.
  #
  # (This only works when sharing the host’s Nix store via 9P, so that
  # the files themselves are already present in `/nix/store` but just
  # not registered in the database; actually copying paths would be
  # slower and more annoying, though it is possible.)
  testBuildClosureInfo =
    config.node.pkgs.runCommand "autopen-test-build-closure-info"
      {
        inherit testDrvPath fakeHash;
        strictDeps = true;
        __structuredAttrs = true;
      }
      ''
        mkdir -- "$out"
        printf "%s\n$fakeHash\n0\n\n0\n" "$NIX_STORE/"* > "$out/registration"
      '';
in

{
  name = "autopen";

  # TODO: This should be able to work with containers if they could use
  # writable stores.
  nodes = {
    machine =
      { ... }:
      {
        nix = {
          settings = {
            substitute = false;
            extra-sandbox-paths = [ "/run/autopen" ];
          };
        };

        systemd.sockets.autopen = {
          wantedBy = [ "multi-user.target" ];
          socketConfig = {
            ListenStream = "/run/autopen/socket";
            SocketGroup = "nixbld";
            SocketMode = "0660";
          };
        };

        systemd.services.autopen = {
          serviceConfig = {
            Type = "exec";
            ExecStart = lib.concatStringsSep " " [
              (lib.getExe autopen)
              "serve"
              "--log"
              "debug"
              "--signing-key-ref"
              remoteSigningKey.fileRefPath
              "\${CREDENTIALS_DIRECTORY}/signing-key"
            ];

            LoadCredential = "signing-key:${softwareSigningKey}";

            ProtectProc = "ptraceable";
            ProcSubset = "pid";
            DynamicUser = true;
            CapabilityBoundingSet = [ "" ];
            UMask = "0077";
            ProtectHome = true;
            PrivateDevices = true;
            PrivateNetwork = true;
            PrivateIPC = true;
            PrivateUsers = "self";
            ProtectHostname = true;
            # `ProtectClock` is redundant to `PrivateDevices`.
            ProtectKernelTunables = true;
            ProtectKernelModules = true;
            ProtectKernelLogs = true;
            ProtectControlGroups = "strict";
            RestrictAddressFamilies = "none";
            RestrictNamespaces = true;
            LockPersonality = true;
            MemoryDenyWriteExecute = true;
            RestrictRealtime = true;
            SystemCallFilter = [
              "@system-service"
              "~@privileged"
              "~@resources"
            ];
            SystemCallArchitectures = "native";
            IPAddressDeny = "any";
          };
        };

        virtualisation.additionalPaths = [
          autopen.tests.software-key.drvPath
        ];
      };
  };

  testScript = ''
    machine.succeed("nix-store --store local --load-db < ${testBuildClosureInfo}/registration")
    machine.wait_for_unit("autopen.socket")
    machine.succeed("nix-build --store daemon --verbose ${testDrvPath}")

    machine.succeed("cp -R --dereference result out")
    machine.copy_from_machine("out", "")
  '';
}
