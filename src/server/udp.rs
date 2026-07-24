use crate::server::state::{TcpMessage, ServerState};

use std::{
    net::{UdpSocket},
    io::{prelude::*, Result},
    sync::{Mutex, Arc},
    thread,
};


pub fn udp_loop(
    udp_socket: UdpSocket,
    udp_state: Arc<Mutex<ServerState>>
)-> Result<()> {
    let mut buf = [0; 10];
    let (amt, src) = udp_socket.recv_from(&mut buf)?;

    let buf = &mut buf[..amt];
    buf.reverse();
    udp_socket.send_to(buf, &src)?;




    thread::spawn(move || {

        // let mut message_to_send;
        // let mut message_bytes;
        //
        // while let Ok(message) = rx.recv() {
        //
        //     match message {
        //
        //         TcpMessage::Authenticated => {
        //             let _ =
        //                 write_stream.write_all(&[0x10]);
        //         }
        //
        //
        //         TcpMessage::UserJoined(name) => {
        //             message_to_send = format!("{name} joined");
        //             message_bytes = message_to_send.as_bytes();
        //
        //             println!("{message_to_send}");
        //
        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //
        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }
        //
        //
        //         TcpMessage::UserLeft(name) => {
        //             message_to_send = format!("{name} left");
        //             message_bytes = message_to_send.as_bytes();
        //
        //             println!("{message_to_send}");
        //
        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //
        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }
        //
        //
        //         TcpMessage::UserStarted(name) => {
        //             message_to_send = format!("{name} started streaming");
        //             message_bytes = message_to_send.as_bytes();
        //
        //             println!("{message_to_send}");
        //
        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //
        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }
        //
        //
        //         TcpMessage::UserStopped(name) => {
        //             message_to_send = format!("{name} stopped streaming");
        //             message_bytes = message_to_send.as_bytes();
        //
        //             println!("{message_to_send}");
        //
        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //
        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }
        //
        //
        //         TcpMessage::Error(err) => {
        //             println!("error: {}", err);
        //         }
        //     }
        // }
    });





    Ok(())
} 
