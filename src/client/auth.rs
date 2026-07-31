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
use crate::protocol::tcp::TcpPacket;
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


    Ok(result[0] == 0x10)
}

pub fn get_uid(
    client_state: &ClientState,
    tcp: &mut TcpStream,
    udp: &UdpSocket,
    udp_addr: SocketAddr,
) -> Result<()>{
    let packet = read_packet(tcp)?;

    match packet {
        TcpPacket::SendUID { uid } => {
            client_state.uid.store(uid, Ordering::Release);

            let packet = UdpPacket::Register { uid };
            udp.send_to(&packet.encode(), udp_addr)?;
            Ok(())
        }

        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected SendUID packet",
            ));
        }
    }
}


pub fn read_packet(stream: &mut TcpStream) -> Result<TcpPacket> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;

    let len = u32::from_be_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;

    TcpPacket::decode(&buf).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid TCP packet",
        )
    })
}
