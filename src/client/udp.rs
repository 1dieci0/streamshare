use std::{
    io::{Result, prelude::*}, net::{SocketAddr, UdpSocket}, sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}}, thread,
};

use crate::{client::ClientState, protocol::{udp::UdpPacket, video::VideoPacket}};
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

            let packet = UdpPacket::Video(
                VideoPacket {
                uid: state.uid,
                sequence: state.sequence,
                data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            });


            let bytes = packet.encode();



            send_socket.send_to(&bytes, udp_addr)?;
            println!("sent");
        
        }


    }


    Ok(())
}
