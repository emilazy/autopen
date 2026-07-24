{ autopen }:

let
  inherit (autopen.lib)
    sign
    ;

  inherit (autopen.lib.internal)
    mkAutopenDerivation
    ;

  inherit (autopen.lib.x509)
    attachSignature
    mkTbsCertificateForSelfSigning
    ;
in
{
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
    attachSignature {
      name = "${name}.crt";
      signature = sign {
        inherit signingKey;
        message = mkTbsCertificateForSelfSigning (
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
}
