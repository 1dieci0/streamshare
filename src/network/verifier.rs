use std::sync::Arc;

use anyhow::{bail, Context, Result};
use rustls::{
    client::danger::{
        HandshakeSignatureValid,
        ServerCertVerified,
        ServerCertVerifier,
    },
    crypto::{
        verify_tls12_signature,
        verify_tls13_signature,
        CryptoProvider,
    },
    pki_types::{
        CertificateDer,
        ServerName,
        UnixTime,
    },
    DigitallySignedStruct,
    Error as RustlsError,
    SignatureScheme,
};
use sha2::{Digest, Sha256};

/// Verifies that the server presents the exact certificate whose
/// SHA-256 fingerprint was distributed to the client.
///
/// This intentionally replaces normal CA and hostname verification.
#[derive(Debug)]
pub struct FingerprintVerifier {
    expected_fingerprint: [u8; 32],
    crypto_provider: Arc<CryptoProvider>,
}

impl FingerprintVerifier {
    pub fn new(fingerprint: &str) -> Result<Arc<Self>> {
        let expected_fingerprint = parse_sha256_fingerprint(fingerprint)
            .context("invalid server certificate fingerprint")?;

        let crypto_provider =
            Arc::new(rustls::crypto::aws_lc_rs::default_provider());

        Ok(Arc::new(Self {
            expected_fingerprint,
            crypto_provider,
        }))
    }

    fn certificate_fingerprint(
        certificate: &CertificateDer<'_>,
    ) -> [u8; 32] {
        Sha256::digest(certificate.as_ref()).into()
    }
}

impl ServerCertVerifier for FingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        let received = Self::certificate_fingerprint(end_entity);

        if received != self.expected_fingerprint {
            return Err(RustlsError::General(format!(
                "server certificate fingerprint mismatch; expected {}, received {}",
                format_sha256_fingerprint(&self.expected_fingerprint),
                format_sha256_fingerprint(&received),
            )));
        }

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self
                .crypto_provider
                .signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self
                .crypto_provider
                .signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.crypto_provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Accepts either:
///
/// AA:BB:CC:...
///
/// or:
///
/// AABBCC...
///
/// Spaces and hyphens are also ignored.
fn parse_sha256_fingerprint(value: &str) -> Result<[u8; 32]> {
    let normalized: String = value
        .chars()
        .filter(|character| {
            !matches!(character, ':' | '-' | ' ' | '\t' | '\n' | '\r')
        })
        .collect();

    if normalized.len() != 64 {
        bail!(
            "SHA-256 fingerprint must contain exactly 64 hexadecimal characters, found {}",
            normalized.len()
        );
    }

    let mut fingerprint = [0_u8; 32];

    for (index, byte) in fingerprint.iter_mut().enumerate() {
        let offset = index * 2;
        let pair = &normalized[offset..offset + 2];

        *byte = u8::from_str_radix(pair, 16).with_context(|| {
            format!("invalid hexadecimal fingerprint byte `{pair}`")
        })?;
    }

    Ok(fingerprint)
}

pub fn format_sha256_fingerprint(
    fingerprint: &[u8; 32],
) -> String {
    fingerprint
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colon_separated_fingerprint() {
        let input = concat!(
            "00:01:02:03:04:05:06:07:",
            "08:09:0A:0B:0C:0D:0E:0F:",
            "10:11:12:13:14:15:16:17:",
            "18:19:1A:1B:1C:1D:1E:1F"
        );

        let parsed = parse_sha256_fingerprint(input).unwrap();

        assert_eq!(parsed[0], 0x00);
        assert_eq!(parsed[31], 0x1F);
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_sha256_fingerprint("AA:BB").is_err());
    }

    #[test]
    fn rejects_invalid_hexadecimal() {
        let input = "ZZ".repeat(32);

        assert!(parse_sha256_fingerprint(&input).is_err());
    }
}