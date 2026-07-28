use crate::server::state::{TcpMessage, ServerState};
use crate::protocol::udp::{UdpPacket};
use crate::protocol::video::{VideoPacket};

use std::{
    io::{Result, prelude::*}, net::UdpSocket, sync::{Arc, RwLock}, thread,
};


pub fn udp_loop(
    udp_socket: UdpSocket,
    state: Arc<RwLock<ServerState>>
)-> Result<()> {
    let recv_socket = udp_socket.try_clone()?;
    let send_socket = udp_socket.try_clone()?;

    let recv_state = Arc::clone(&state);
    let send_state = Arc::clone(&state);

    thread::spawn(move || {
        let mut buf = [0; 1024];
        loop{
            let (len, addr) = recv_socket.recv_from(&mut buf).unwrap();

            
            let Some(packet) = UdpPacket::decode(&buf[..len]) else {
                return;
            };

            match packet{
                UdpPacket::Register { uid } => {
                    let mut state = recv_state.write().unwrap();
                    let Some(user) = state.users.get_mut(&uid) else{
                        return;
                    };
                    println!("{}", addr);
                    user.udp_addr = Some(addr);
                }
                UdpPacket::Video(video) => {
                    println!("lol");
                    let mut state = recv_state.write().unwrap();
                    if !state.streams.contains_key(&video.uid){
                        continue;
                    }
                    let Some(stream) = state.streams.get_mut(&video.uid) else{
                        return;
                    };
                    println!("{:?}", video.data);
                    stream.latest_packet = Some(video);
                }
                UdpPacket::Heartbeat => {}
            }
            
        }
        
    });


    loop{
        let state = send_state.read().unwrap();

        for (&stream_uid, stream) in &state.streams {
            let Some(packet) = &stream.latest_packet else {
                continue;
            };

            let bytes = packet.encode();


            for (&client_uid, client) in &state.users {
                if client_uid == stream_uid {
                    continue; 
                }

                let Some(addr) = client.udp_addr else {
                    println!("diobono");
                    continue;
                };

                println!("ho quasi mandato");
                send_socket.send_to(&bytes, addr)?;
                
            }
        }
    }


    Ok(())
} 