use std::net::SocketAddr;
use std::net::TcpStream;
use std::io::prelude::*;
use std::io::Result;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::Ordering;
use sha2::{Digest, Sha256};

use crate::client::state::ClientState;
use crate::protocol::udp::UdpPacket;

pub const SERVER_KEY: &str = "super_secret_key";

fn create_response(key: &str, challenge: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();

    hasher.update(key.as_bytes());
    hasher.update(challenge);

    hasher.finalize().to_vec()
}

pub fn authenticate(
    tcp: &mut TcpStream,
    username: &str,
) -> Result<bool> {

    // Receive challenge
    let mut challenge = [0u8; 32];

    tcp.read_exact(&mut challenge)?;


    // Create response
    let response = create_response(
        SERVER_KEY,
        &challenge,
    );


    // Send username length
    let username_bytes = username.as_bytes();

    let len = username_bytes.len() as u16;

    tcp.write_all(
        &len.to_be_bytes()
    )?;


    // Send username
    tcp.write_all(username_bytes)?;


    // Send hash response
    tcp.write_all(&response)?;


    // Receive server answer
    let mut result = [0u8; 1];

    tcp.read_exact(&mut result)?;


    Ok(result[0] == 0x01)
}

pub fn get_uid(
    client_state: &ClientState,
    tcp: &mut TcpStream,
    udp: &UdpSocket,
    udp_addr: SocketAddr,
) -> Result<()>{
        let mut buf = [0u8; 8];
        tcp.read_exact(&mut buf)?;

        let uid = u64::from_be_bytes(buf);

        
        //client_state.write().unwrap().uid = uid;
        client_state.uid.store(uid, Ordering::Release);

        

        let packet = UdpPacket::Register { uid: (uid) };

        let bytes = packet.encode();

        udp.send_to(&bytes, udp_addr)?;

        Ok(())
}

