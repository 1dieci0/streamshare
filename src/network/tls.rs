use anyhow::Result;
use rustls::pki_types::{
    CertificateDer,
    PrivateKeyDer,
    PrivatePkcs8KeyDer,
};

use std::{
    fs,
    path::Path,
};

use rcgen::generate_simple_self_signed;


pub struct ServerCert {
    pub cert_chain: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}


pub fn load_or_generate_server_cert(
    cert_path: &Path,
    key_path: &Path,
) -> Result<ServerCert> {


    if cert_path.exists() && key_path.exists() {

        let cert =
            CertificateDer::from(
                fs::read(cert_path)?
            );


        let key =
            PrivateKeyDer::from(
                PrivatePkcs8KeyDer::from(
                    fs::read(key_path)?
                )
            );


        return Ok(ServerCert {
            cert_chain: vec![cert],
            key,
        });
    }


    println!("Generating new server certificate");


    let generated =
        generate_simple_self_signed(
            vec![
                "streamshare".into()
            ]
        )?;


    let cert =
        generated.cert.der().to_vec();


    let key =
        generated
        .signing_key
        .serialize_der();


    fs::write(
        cert_path,
        &cert
    )?;

    fs::write(
        key_path,
        &key
    )?;


    Ok(ServerCert {
        cert_chain: vec![
            CertificateDer::from(cert)
        ],

        key:
            PrivatePkcs8KeyDer::from(key)
            .into(),
    })
}