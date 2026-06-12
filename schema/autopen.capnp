# SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
#
# SPDX-License-Identifier: BlueOak-1.0.0

@0x84bf4b5f6bdc07b4;

interface Signer {
    # Access to a cryptographic signing key.

    sign @0 (message :Data) -> (signature :Data);
    # Signs a message and returns a detached signature.

    getVerificationKey @1 () -> (verificationKey :VerificationKey);
    # Returns the verification key corresponding to the signing key.
}

struct VerificationKey {
    # A persisted verification key.

    union {
        rsa3072Pkcs1Sha256 @0 :Rsa3072Pkcs1Sha256;
        reserved @1 :Void;
    }

    struct Rsa3072Pkcs1Sha256 {
        # An `rsa3072-pkcs1-sha256` verification key.

        pkcs1Der @0 :Data;
        # The ASN.1 DER encoding of an `RSAPublicKey`, as defined in
        # [Appendix A.1.1 of RFC 8017].
        #
        # [Appendix A.1.1 of RFC 8017]:
        # <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.1>
    }
}

struct SigningKey {
    # A persisted signing key.

    union {
        software @0 :Software;
        remote @1 :Remote;
    }

    struct Software {
        # A software signing key.

        union {
            rsa3072Pkcs1Sha256 @0 :Rsa3072Pkcs1Sha256;
            reserved @1 :Void;
        }

        struct Rsa3072Pkcs1Sha256 {
            # An `rsa3072-pkcs1-sha256` software signing key.

            pkcs1Der @0 :Data;
            # The ASN.1 DER encoding of an `RSAPrivateKey`, as defined
            # in [Appendix A.1.2 of RFC 8017].
            #
            # [Appendix A.1.2 of RFC 8017]:
            # <https://www.rfc-editor.org/info/rfc8017/#appendix-A.1.2>
        }
    }

    struct Remote {
        # A signing key accessed through a server.

        remoteRef @0 :Local.RemoteRef;
        # A persistent reference to the remote signing key.

        verificationKey @1 :VerificationKey;
        # The verification key corresponding to the remote signing key.
    }
}

interface Restorer(Ref) {
    # Access to restore sturdy references in a given per‐realm format.

    restore @0 (sturdyRef :Ref) -> (cap :Capability);
    # Restores a sturdy reference to a capability.
}

interface Bootstrap(Ref) {
    # The bootstrap interface for a context.

    getRestorer @0 () -> (restorer :Restorer(Ref));
    # Returns a restorer for sturdy references.
}

interface UnixSocketServer extends(Bootstrap(FileRef)) {
    # A Unix socket server.

    interface FileRef {
        # A persistent reference to an object, identified by a POSIX
        # file identity and passed between vats as a file descriptor
        # with read access.
    }
}

interface Local extends(Bootstrap(RemoteRef)) {
    # A local context.

    struct RemoteRef {
        # A persistent reference to an object on a remote server.

        socketPath @0 :Text;
        # The path to the server’s Unix socket.

        fileRefPath @1 :Text;
        # The path to the file reference corresponding to the remote object.
    }

    struct File {
        # The top‐level structure of a persisted file.

        union {
            verificationKey @0 :VerificationKey;
            signingKey @1 :SigningKey;
        }
    }
}
