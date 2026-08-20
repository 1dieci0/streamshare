// src/server/endpoint.rs

use std::fs;
use std::{error::Error, path::Path};
use std::net::SocketAddr;
use std::sync::Arc;
use quinn::crypto::rustls::QuicServerConfig;
use sha2::{Digest, Sha256};
use quinn::{Endpoint, ServerConfig};
use anyhow::{Context, Result};

use crate::network::quic::ALPN_PROTOCOL;
use crate::{network::quic::transport_config, server};

/// Builds and binds a QUIC server endpoint with a self-signed certificate.
///
/// Generates a fresh self-signed cert/key pair on each startup, suitable
/// for dev use or apps where clients pin the cert out-of-band (e.g. via
/// a shared code/fingerprint) rather than relying on a CA chain.
pub fn make_server_endpoint(config: server::config::ServerConfig) -> Result<Endpoint> {
    let bind_addr = config.network.socket_addr()?;

    let (server_config, cert_der) = configure_self_signed_server(config.tls.certificate, config.tls.private_key)?;

    let endpoint = Endpoint::server(server_config, bind_addr)?;

    println!("Server listening on {bind_addr}");
    println!("Share this fingerprint with clients:");
    println!("  {}", cert_fingerprint(&cert_der));

    Ok(endpoint)
}
/// Generates a self-signed cert and builds the rustls-backed ServerConfig.
///
/// Returns the ServerConfig plus the raw certificate DER, in case you want
/// to print/share the cert fingerprint with clients for pinning.
fn configure_self_signed_server(
    cert: String,
    key: String,
) -> Result<(ServerConfig, Vec<u8>)> {

    let cert_path = Path::new(&cert);
    let key_path = Path::new(&key);

    let (cert_der, private_key) = 
        if cert_path.exists() && key_path.exists(){
            println!("Loading existing TLS certificate");
            
            
            let cert_der =
                fs::read(cert_path)
                    .context("failed reading certificate")?;

            let key_der =
                fs::read(key_path)
                    .context("failed reading private key")?;

            let private_key =
                rustls::pki_types::PrivateKeyDer::try_from(key_der)
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "invalid private key: {e}"
                        )
                    })?;

            (
                cert_der,
                private_key,
            )
        
        }else{
            println!("TLS certificate not found");
            println!("Generating self signed TLS certificate");

            let cert =
                rcgen::generate_simple_self_signed(
                    vec!["localhost".into()]
                )?;
            

            let cert_der =
                cert.cert.der().to_vec();

            let key_der =
                cert.signing_key.serialize_der();
            

            fs::write(
                cert_path,
                &cert_der,
            )
            .context("failed writing certificate")?;


            fs::write(
                key_path,
                &key_der,
            )
            .context("failed writing private key")?;


            let private_key =
                rustls::pki_types::PrivateKeyDer::Pkcs8(
                    rustls::pki_types::PrivatePkcs8KeyDer::from(key_der)
                );

            (
                cert_der,
                private_key,
            )
        };

    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der.clone())];

    //let mut server_config = ServerConfig::with_single_cert(cert_chain, private_key)?;


    let mut crypto =
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                cert_chain,
                private_key,
            )?;

    crypto.alpn_protocols = vec![
        ALPN_PROTOCOL.to_vec()
    ];

    let mut server_config =
        ServerConfig::with_crypto(
            Arc::new(
                QuicServerConfig::try_from(
                    crypto
                )?
            )
        );



    server_config.transport_config(Arc::new(transport_config()));

    Ok((server_config, cert_der))
}



/// Computes a SHA-256 fingerprint of the cert and formats it as a
/// colon-separated hex string, e.g. "A3:F1:9C:...:4B" — short enough to
/// read aloud or paste into a chat.
pub fn cert_fingerprint(cert_der: &[u8]) -> String {
    let hash = Sha256::digest(cert_der);
    hash.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}