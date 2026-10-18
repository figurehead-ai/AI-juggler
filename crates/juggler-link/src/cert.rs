//! TLS certificates derived from the machine identity (AJ-E703).
//!
//! Each node presents a self-signed X.509 certificate whose
//! `SubjectPublicKeyInfo` IS its Ed25519 identity key. There is no CA:
//! peers accept a certificate only when that key is pinned
//! ([`crate::verify`]), and TLS 1.3's `CertificateVerify` proves the
//! sender holds the private half.

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::identity::MachineIdentity;

/// The fixed PKCS#8 v1 prefix for an Ed25519 private key (RFC 8410):
/// SEQUENCE { INTEGER 0, `AlgorithmIdentifier` { `id-Ed25519` },
/// OCTET STRING { OCTET STRING (32 bytes) } }.
const ED25519_PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// The `id-Ed25519` OID (1.3.101.112).
const ED25519_OID: &[u64] = &[1, 3, 101, 112];

/// Certificate failures.
#[derive(Debug, thiserror::Error)]
pub enum CertError {
    /// The certificate could not be built.
    #[error("could not build this Mac's link certificate: {0}")]
    Build(String),
    /// A presented certificate is not a single Ed25519 key certificate.
    #[error("the peer's certificate is not an Ed25519 identity certificate")]
    NotEd25519,
}

/// This node's certificate chain (one self-signed cert) and key.
pub struct NodeCertificate {
    /// The DER certificate.
    pub cert: CertificateDer<'static>,
    /// The PKCS#8 private key (the identity seed). Never logged.
    pub key: PrivateKeyDer<'static>,
}

impl std::fmt::Debug for NodeCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeCertificate").finish_non_exhaustive()
    }
}

/// Builds the self-signed certificate for `identity`.
///
/// # Errors
///
/// Returns [`CertError::Build`] when the certificate cannot be encoded.
pub fn node_certificate(identity: &MachineIdentity) -> Result<NodeCertificate, CertError> {
    let mut pkcs8 = ED25519_PKCS8_PREFIX.to_vec();
    pkcs8.extend_from_slice(&identity.signing_seed());
    let key_der = PrivatePkcs8KeyDer::from(pkcs8);
    let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&key_der, &rcgen::PKCS_ED25519)
        .map_err(|e| CertError::Build(e.to_string()))?;
    // The name is cosmetic — identity is the key, never the name.
    let mut params = rcgen::CertificateParams::new(vec!["aijuggler.node".to_owned()])
        .map_err(|e| CertError::Build(e.to_string()))?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, identity.public().fingerprint());
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| CertError::Build(e.to_string()))?;
    Ok(NodeCertificate {
        cert: cert.der().clone(),
        key: PrivateKeyDer::Pkcs8(key_der),
    })
}

/// The Ed25519 public key a certificate carries in its
/// `SubjectPublicKeyInfo`.
///
/// # Errors
///
/// Returns [`CertError::NotEd25519`] for anything else.
pub fn ed25519_key_of(cert: &CertificateDer<'_>) -> Result<[u8; 32], CertError> {
    let (_, parsed) =
        x509_parser::parse_x509_certificate(cert.as_ref()).map_err(|_| CertError::NotEd25519)?;
    let spki = parsed.public_key();
    let oid: Vec<u64> = spki
        .algorithm
        .algorithm
        .iter()
        .map(Iterator::collect)
        .unwrap_or_default();
    if oid != ED25519_OID {
        return Err(CertError::NotEd25519);
    }
    spki.subject_public_key
        .data
        .as_ref()
        .try_into()
        .map_err(|_| CertError::NotEd25519)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_certificate_carries_exactly_the_identity_key() {
        let dir = tempfile::tempdir().unwrap();
        let identity = MachineIdentity::load_or_create(&dir.path().join("fleet")).unwrap();
        let node = node_certificate(&identity).unwrap();
        assert_eq!(
            ed25519_key_of(&node.cert).unwrap(),
            identity.public().to_bytes()
        );
        assert!(
            !format!("{node:?}").contains("key:"),
            "Debug never prints the key"
        );
    }

    #[test]
    fn a_non_ed25519_certificate_is_refused() {
        let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
        let cert = rcgen::CertificateParams::new(vec!["x".into()])
            .unwrap()
            .self_signed(&key)
            .unwrap();
        assert!(matches!(
            ed25519_key_of(cert.der()),
            Err(CertError::NotEd25519)
        ));
        assert!(ed25519_key_of(&CertificateDer::from(vec![1, 2, 3])).is_err());
    }
}
