use serde::{Deserialize, Serialize};
use std::{
    fs, net::SocketAddr, path::{Path, PathBuf},
};

use anyhow::{bail, Result};



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {

    pub server_name: String,

    pub network: NetworkConfig,

    pub tls: TlsConfig,

    pub media: MediaConfig,

    pub security: SecurityConfig,
}



#[derive(Debug, Serialize, Deserialize, Clone)]pub struct NetworkConfig {

    pub bind_address: String,

    pub quic_port: u16,
}



impl NetworkConfig {

    pub fn socket_addr(
        &self
    ) -> Result<SocketAddr> {

        Ok(format!(
            "{}:{}",
            self.bind_address,
            self.quic_port
        ).parse()?)
    }
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TlsConfig {

    pub certificate: String,

    pub private_key: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaConfig {

    pub max_width: u32,

    pub max_height: u32,

    pub max_fps: u32,

    pub max_streams: usize,
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {

    pub require_authentication: bool,
    pub password: String,
}



impl Default for ServerConfig {

    fn default() -> Self {

        Self {

            server_name:
                "localhost".into(),


            network:
                NetworkConfig {
                    bind_address:
                        "0.0.0.0".into(),

                    quic_port:
                        5001,
                },


            tls:
                TlsConfig {
                    certificate:
                        "certs/server_cert.der".into(),

                    private_key:
                        "certs/server_key.der".into(),
                },


            media:
                MediaConfig {
                    max_width:
                        1920,

                    max_height:
                        1080,

                    max_fps:
                        60,

                    max_streams:
                        10,
                },


            security:
                SecurityConfig {
                    require_authentication:
                        true,
                    password: "password".into(),
                },
        }
    }
}



impl ServerConfig {


    pub fn load_or_create(
        path: impl AsRef<Path>
    ) -> Result<Self> {

        let path = path.as_ref();


        if !path.exists() {

            let default =
                ServerConfig::default();


            let json =
                serde_json::to_string_pretty(
                    &default
                )?;


            fs::write(
                path,
                json
            )?;


            bail!(
                "Created server config at {:?}. Edit it before starting.",
                path
            );
        }


        let data =
            fs::read_to_string(path)?;


        let config:
            ServerConfig =
            serde_json::from_str(
                &data
            )?;


        config.validate()?;


        Ok(config)
    }



    fn validate(
        &self
    ) -> Result<()> {


        if self.server_name.trim().is_empty() {
            bail!("Missing server name");
        }


        if self.network.quic_port == 0 {
            bail!("Invalid QUIC port");
        }


        if self.tls.certificate.is_empty() {
            bail!("Missing certificate path");
        }


        if self.tls.private_key.is_empty() {
            bail!("Missing private key path");
        }


        Ok(())
    }
}