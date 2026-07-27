use std::{
    io::{Result, prelude::*}, net::{SocketAddr, UdpSocket}, sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}}, thread,
};

use crate::{client::ClientState, protocol::VideoPacket};
use crate::client::Client;

pub fn udp_loop(
    state: Arc<RwLock<ClientState>>,
    udp_addr: SocketAddr,
    udp_socket: UdpSocket,
)-> Result<()> {

    let recv_socket = udp_socket.try_clone()?;
    let send_socket = udp_socket.try_clone()?;

    thread::spawn(move || {
        let mut buf = [0; 1024];
        loop{
            let (len, addr) = recv_socket.recv_from(&mut buf).unwrap();

            let Some(packet) = VideoPacket::decode(&buf[..len]) else {
                return;
            };

            println!("received {:?}", packet.data);
        }
        
    });


    loop{

        let state = state.read().unwrap();

        if state.streaming.load(Ordering::Relaxed) {
            let packet = VideoPacket{
                uid: state.uid,
                sequence: state.uid,
                data: Vec::new(),
            };

            let bytes = packet.encode();



            send_socket.send_to(&bytes, udp_addr)?;
            println!("sent");
        
        }


    }


    Ok(())
}
