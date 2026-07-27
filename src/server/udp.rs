use crate::server::state::{TcpMessage, ServerState};
use crate::protocol::{VideoPacket};

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

            //println!("{:?}", &buf);
            
            let Some(packet) = VideoPacket::decode(&buf[..len]) else {
                return;
            };

            println!("diobono");

            let uid = packet.uid;

            let mut state = recv_state.write().unwrap();

            if !state.streams.contains_key(&uid){
                continue;
            }

            println!("diobono");

            let Some(stream) = state.streams.get_mut(&uid) else{
                return;
            };

            println!("diobono");

            stream.latest_packet = Some(packet);

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
                    continue;
                };

                send_socket.send_to(&bytes, addr)?;
            }
        }
    }


    Ok(())
} 



// #[test]
// fn client_server() {
//     let server = thread::spawn(||  {
//         let socket = UdpSocket::bind("127.0.0.1:40000").unwrap();
//
//         let mut buf = [0; 1024];
//         let (len, addr) = socket.recv_from(&mut buf).unwrap();
//
//         assert_eq!(&buf[..len], b"ping");
//
//         socket.send_to(b"pong", addr).unwrap();
//     });
//
//     thread::sleep(Duration::from_millis(50));
//
//     let client = UdpSocket::bind("127.0.0.1:0").unwrap();
//
//     client.send_to(b"ping", "127.0.0.1:40000").unwrap();
//
//     let mut buf = [0; 1024];
//     let (len, _) = client.recv_from(&mut buf).unwrap();
//
//     assert_eq!(&buf[..len], b"pong");
//
//     server.join().unwrap();
// }