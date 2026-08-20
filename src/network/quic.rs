// src/network/quic.rs

use std::time::Duration;

use quinn::{TransportConfig, VarInt};

/// ALPN protocol identifier — must match exactly on client and server
/// or the TLS handshake will fail.
pub const ALPN_PROTOCOL: &[u8] = b"streamshare/1";

/// Shared QUIC transport settings. Client and server should both use this
/// so keepalive/timeout/stream-limit behavior stays in sync — mismatches
/// here are a common source of "works sometimes" bugs.
pub fn transport_config() -> TransportConfig {
    let mut config = TransportConfig::default();

    // Keep the connection alive through NATs/idle periods.
    config.keep_alive_interval(Some(Duration::from_secs(5)));
    config.max_idle_timeout(Some(
        Duration::from_secs(30)
            .try_into()
            .expect("valid idle timeout"),
    ));

    // Tune concurrent stream limits — raise these if you plan on
    // multiplexing many simultaneous streams (e.g. multi-viewer).
    config.max_concurrent_uni_streams(VarInt::from_u32(12));
    config.max_concurrent_bidi_streams(VarInt::from_u32(12));

    config
}