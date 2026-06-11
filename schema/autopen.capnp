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
        reserved @1 :Void;
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
}

interface Local {
    # A local context.

    struct File {
        # The top‐level structure of a persisted file.

        union {
            verificationKey @0 :VerificationKey;
            signingKey @1 :SigningKey;
        }
    }
}
