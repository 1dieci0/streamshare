use std::{
    fs,
    net::SocketAddr,
    path::Path,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::network::verifier::FingerprintVerifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub username: String,
    pub server_name: String,
    pub server_address: String,
    pub server_password: String,
    pub fingerprint: String,
}

impl ClientConfig {
    pub fn load_or_create(
        path: impl AsRef<Path>,
    ) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            let template = Self {
                username: "your_username".into(),
                server_name: "localhost".into(),
                server_address: "127.0.0.1:5001".into(),
                server_password: "password".into(),
                fingerprint: String::new(),
            };

            let json = serde_json::to_string_pretty(&template)?;
            fs::write(path, json)?;

            bail!(
                "created client configuration template at {}; fill in the server fingerprint and restart",
                path.display()
            );
        }

        let contents = fs::read_to_string(path)
            .with_context(|| {
                format!(
                    "failed to read client config {}",
                    path.display()
                )
            })?;

        let config: Self = serde_json::from_str(&contents)
            .with_context(|| {
                format!(
                    "invalid client config JSON in {}",
                    path.display()
                )
            })?;

        config.validate()?;

        Ok(config)
    }

    pub fn server_addr(&self) -> Result<SocketAddr> {
        self.server_address.parse().with_context(|| {
            format!(
                "invalid server address `{}`",
                self.server_address
            )
        })
    }

    fn validate(&self) -> Result<()> {
        if self.username.trim().is_empty()
            || self.username == "your_username"
        {
            bail!("client username has not been configured");
        }

        if self.server_name.trim().is_empty() {
            bail!("server_name is missing");
        }

        if self.server_address.trim().is_empty() {
            bail!("server_address is missing");
        }

        if self.server_password.trim().is_empty() {
            bail!("server_password is missing");
        }

        if self.fingerprint.trim().is_empty() {
            bail!("trusted server fingerprint is missing");
        }

        // Validate it while loading rather than during connection.
        FingerprintVerifier::new(&self.fingerprint)?;

        Ok(())
    }
}