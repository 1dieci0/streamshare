use std::net::TcpStream;
use std::io::prelude::*;
use std::io::Result;
use sha2::{Digest, Sha256};

pub const SERVER_KEY: &str = "super_secret_key";

fn create_response(key: &str, challenge: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();

    hasher.update(key.as_bytes());
    hasher.update(challenge);

    hasher.finalize().to_vec()
}

pub fn authenticate(
    stream: &mut TcpStream,
    username: &str,
) -> Result<bool> {

    // Receive challenge
    let mut challenge = [0u8; 32];

    stream.read_exact(&mut challenge)?;


    // Create response
    let response = create_response(
        SERVER_KEY,
        &challenge,
    );


    // Send username length
    let username_bytes = username.as_bytes();

    let len = username_bytes.len() as u16;

    stream.write_all(
        &len.to_be_bytes()
    )?;


    // Send username
    stream.write_all(username_bytes)?;


    // Send hash response
    stream.write_all(&response)?;


    // Receive server answer
    let mut result = [0u8; 1];

    stream.read_exact(&mut result)?;


    Ok(result[0] == 0x01)
}



