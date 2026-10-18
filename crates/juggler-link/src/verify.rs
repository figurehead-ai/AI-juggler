//! Pinned-key certificate verification (AJ-E703; ADR-0013 contract).
//!
//! There is no CA. A server is accepted only when its certificate's
//! `SubjectPublicKeyInfo` is exactly the expected (pinned) Ed25519 key;
//! the TLS 1.3 `CertificateVerify` signature is still checked, so
//! presenting someone's public key without the private half fails.
//! Clients must present an Ed25519 certificate; the server then
//! authorises each call by looking the key up in the peer registry —
//! an unpinned key reaches only the pairing exchange, which grants
//! nothing until it succeeds. TLS 1.2 signatures are always refused.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, Error, SignatureScheme};

use crate::cert::ed25519_key_of;

/// The crypto provider every link connection uses (ring).
#[must_use]
pub fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn single_ed25519(
    end_entity: &CertificateDer<'_>,
    intermediates: &[CertificateDer<'_>],
) -> Result<[u8; 32], Error> {
    if !intermediates.is_empty() {
        return Err(Error::InvalidCertificate(CertificateError::BadEncoding));
    }
    ed25519_key_of(end_entity).map_err(|_| Error::InvalidCertificate(CertificateError::BadEncoding))
}

fn tls13(
    provider: &CryptoProvider,
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &DigitallySignedStruct,
) -> Result<HandshakeSignatureValid, Error> {
    if dss.scheme != SignatureScheme::ED25519 {
        return Err(Error::PeerIncompatible(
            rustls::PeerIncompatible::NoSignatureSchemesInCommon,
        ));
    }
    rustls::crypto::verify_tls13_signature(
        message,
        cert,
        dss,
        &provider.signature_verification_algorithms,
    )
}

fn no_tls12() -> Result<HandshakeSignatureValid, Error> {
    Err(Error::General(
        "TLS 1.2 is not supported on the Fleet link".to_owned(),
    ))
}

/// Who the client expects the server to be.
#[derive(Clone, Copy, Debug)]
pub enum Expected {
    /// Exactly this pinned Ed25519 key (every normal connection).
    Pinned([u8; 32]),
    /// Any Ed25519 key — ONLY for the pairing exchange, which binds the
    /// key it saw into the SPAKE2 confirmation before anything is pinned.
    AnyForPairing,
}

/// The client-side verifier.
#[derive(Debug)]
pub struct ServerKeyVerifier {
    expected: Expected,
    provider: Arc<CryptoProvider>,
}

impl ServerKeyVerifier {
    /// A verifier expecting `expected`.
    #[must_use]
    pub fn new(expected: Expected) -> Self {
        Self {
            expected,
            provider: provider(),
        }
    }
}

impl ServerCertVerifier for ServerKeyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let key = single_ed25519(end_entity, intermediates)?;
        match self.expected {
            Expected::Pinned(pinned) if pinned == key => Ok(ServerCertVerified::assertion()),
            Expected::Pinned(_) => Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            )),
            Expected::AnyForPairing => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        no_tls12()
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        tls13(&self.provider, message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

/// The server-side verifier: the client MUST present an Ed25519
/// certificate (authorisation happens per call against the registry).
#[derive(Debug)]
pub struct ClientKeyVerifier {
    provider: Arc<CryptoProvider>,
}

impl ClientKeyVerifier {
    /// The verifier.
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: provider(),
        }
    }
}

impl Default for ClientKeyVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientCertVerifier for ClientKeyVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        single_ed25519(end_entity, intermediates)?;
        Ok(ClientCertVerified::assertion())
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        no_tls12()
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        tls13(&self.provider, message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}
