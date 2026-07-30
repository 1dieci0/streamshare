use crate::server::state::{ServerState};
use crate::protocol::udp::{UdpPacket};
use crate::protocol::video::{VideoPacket};

use std::{
    io::{Result, prelude::*}, net::UdpSocket, sync::{Arc, RwLock}, thread,
};


pub fn udp_loop(
    udp_socket: UdpSocket,
    state: Arc<RwLock<ServerState>>
)-> Result<()> {

    thread::spawn(move || {
        let mut buf = [0; 1500];
        loop{
            let (len, addr) = match udp_socket.recv_from(&mut buf){
                Ok(v) => v,
                Err(e) => {
                    eprint!("UDP receive error: {e}");
                    continue;
                }
            };

            
            let Some(packet) = UdpPacket::decode(&buf[..len]) else {
                return;
            };

            match packet{
                UdpPacket::Register { uid } => {
                    let mut state = state.write().unwrap();

                    if let Some(user) = state.users.get_mut(&uid){
                       user.udp_addr = Some(addr);
                    };
                }


                UdpPacket::Video(video) => {

                    let recipients = {
                        let state = state.read().unwrap();

                        if !state.streams.contains_key(&video.uid){
                            continue;
                        }


                        state.users
                            .iter()
                            .filter_map(|(&uid, user)| {
                                if uid == video.uid {
                                    None
                                } else {
                                  user.udp_addr
                                }
                            })
                            .collect::<Vec<_>>()
                    };

                    let bytes = UdpPacket::Video(video).encode();

                    for addr in recipients{
                        if let Err(e) = udp_socket.send_to(&bytes, addr){
                            eprint!("UDP send error: {e}");
                        }
                    }
                }
                UdpPacket::Heartbeat => {}
            }
            
        }
        
    });

    
    Ok(())
} 