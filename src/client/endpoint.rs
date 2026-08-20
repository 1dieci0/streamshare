// src/client/endpoint.rs

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use sha2::{Digest, Sha256};
use quinn::{ClientConfig, Endpoint};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};

use crate::network::quic::{transport_config, ALPN_PROTOCOL};

/// Builds a client endpoint that trusts exactly one pinned certificate —
/// the one the server shared out-of-band (e.g. printed at startup,
/// exchanged via a room code, etc). Use this instead of a CA-based trust
/// chain when the server cert is self-signed.
pub fn make_client_endpoint(
    bind_addr: SocketAddr,
    expected_fingerprint: String,
) ->anyhow::Result<Endpoint> {
    let fingerprint_bytes = parse_fingerprint(expected_fingerprint)?;
    let client_config = configure_pinned_client(fingerprint_bytes)?;

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    Ok(endpoint)
}

/// Connects to the server and returns the established connection.
pub async fn connect(
    endpoint: &Endpoint,
    server_addr: SocketAddr,
    server_name: &str,
) -> anyhow::Result<quinn::Connection> {
    let connection = endpoint.connect(server_addr, server_name)?.await?;
    Ok(connection)
}

fn configure_pinned_client(
    expected_fingerprint: Vec<u8>
) -> anyhow::Result<ClientConfig> {
    let verifier = FingerprintCertVerifier { expected_fingerprint };

    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();

    crypto.alpn_protocols = vec![ALPN_PROTOCOL.to_vec()];

    let mut client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    ));
    client_config.transport_config(Arc::new(transport_config()));

    Ok(client_config)
}

/// Verifier that accepts a connection only if the presented cert's
/// SHA-256 fingerprint matches the one the user entered (shared out-of-band).
#[derive(Debug)]
struct FingerprintCertVerifier {
    expected_fingerprint: Vec<u8>,
}

impl ServerCertVerifier for FingerprintCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let actual = Sha256::digest(end_entity.as_ref());
        if actual.as_slice() == self.expected_fingerprint.as_slice() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "server certificate fingerprint does not match expected value".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}



/// Parses a fingerprint string like "A3:F1:9C:...:4B" (or without colons,
/// case-insensitive) into raw bytes for comparison.
pub fn parse_fingerprint(
    input: String
) -> anyhow::Result<Vec<u8>> {
    let cleaned: String = input.chars().filter(|c| *c != ':' && *c != ' ').collect();
    if cleaned.len() != 64 {
        return Err(anyhow::anyhow!("fingerprint must be a 32-byte SHA-256 hash (64 hex chars)"));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).map_err(|e| e.into()))
        .collect()
}