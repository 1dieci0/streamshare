use crate::protocol;
use crate::protocol::tcp::TcpPacket;
use crate::server::auth::{generate_challenge, create_response, SERVER_KEY};
use crate::server::state::{ServerState, Client, Stream, generate_session_id};

use std::{
    net::{TcpStream},
    io::{prelude::*,Result},
    sync::{RwLock, Arc},
    thread,
    sync::mpsc::{channel},
};


pub fn handle_tcp(
    mut stream: TcpStream,
    state: Arc<RwLock<ServerState>>,
) -> Result<()> {

    /*
        AUTHENTICATION PHASE
    */

    let challenge = generate_challenge();

    // Send challenge
    stream.write_all(&challenge)?;


    // Receive username length
    let mut username_len = [0u8; 2];
    stream.read_exact(&mut username_len)?;

    let username_len =
        u16::from_be_bytes(username_len) as usize;


    // Receive username
    let mut username_buf = vec![0u8; username_len];
    stream.read_exact(&mut username_buf)?;

    let username =
        String::from_utf8(username_buf)
            .unwrap();

    let uid: u64;


    // Receive hash response
    let mut response = [0u8; 32];
    stream.read_exact(&mut response)?;


    // Verify
    let expected =
        create_response(
            SERVER_KEY,
            &challenge,
        );


    if response != expected[..] {
        stream.write_all(&TcpPacket::AuthDenied.encode())?;
        println!("{username} failed authentication");
        return Ok(());
    }


    stream.write_all(&TcpPacket::AuthAccepted.encode())?;
    println!("{username} authenticated");


    /*
        CREATE MPSC
    */

    let (tx, rx) =
        channel::<TcpPacket>();


    /*
        WRITER THREAD
    */

    let mut write_stream =
        stream.try_clone()?;


    thread::spawn(move || {
        while let Ok(packet) = rx.recv() {
            let bytes = packet.encode();

            if let Err(e) = write_stream.write_all(&(bytes.len() as u32).to_be_bytes()) {
                println!("Connection closed: {e}");
                break;
            }

            if let Err(e) = write_stream.write_all(&bytes) {
                println!("Connection closed: {e}");
                break;
            }
        }


        // let mut message_to_send;
        // let mut message_bytes;

        // while let Ok(message) = rx.recv() {

        //     match message {

        //         TcpPacket::Authenticated => {
        //             let _ =
        //                 write_stream.write_all(&[0x10]);
        //         }

        //         TcpPacket::SendUID{uid} => {
        //             let bytes = uid.to_be_bytes();
        //             let _ = write_stream.write_all(&bytes);
        //         }


        //         TcpPacket::UserJoined{
        //             uid,
        //             username
        //         } => {
        //             message_to_send = format!("{username} joined");
        //             message_bytes = message_to_send.as_bytes();

        //             println!("{message_to_send}");

        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }

        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }


        //         TcpPacket::UserLeft{
        //             uid,
        //             username
        //         } => {
        //             message_to_send = format!("{username} left");
        //             message_bytes = message_to_send.as_bytes();

        //             println!("{message_to_send}");

        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }

        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }


        //         TcpPacket::StreamStarted{
        //             uid,
        //             username
        //         } => {
        //             let packet = protocol::tcp::TcpPacket::UserJoined {uid, username: username.clone()};
        //             let bytes = packet.encode();
        //             message_to_send = format!("{username} started streaming");
        //             message_bytes = message_to_send.as_bytes();
                    
        //             println!("{message_to_send}");

        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }

        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }


        //         TcpPacket::StreamStopped{
        //             uid, 
        //             username
        //         } => {
        //             message_to_send = format!("{username} stopped streaming");
        //             message_bytes = message_to_send.as_bytes();

        //             println!("{message_to_send}");

        //             if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }

        //             if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
        //                 println!("Connection closed: {e}");
        //                 break;
        //             }
        //         }


        //         TcpPacket::Error{error} => {
        //             println!("error: {}", error);
        //         }
        //     }
        // }
    });



    /*
        REGISTER CLIENT
    */

    {
        let mut state =
            state.write().unwrap();

        uid = generate_session_id(&state);

        state.users.insert(
            uid,
            Client {
                username: username.clone(),
                session_id: uid,
                tcp_sender: tx.clone(),
                udp_addr: None,
            }
        );

        // notify existing users
        for (client_uid, client)
            in &state.users
        {
            if client_uid != &uid {
                let _ =
                    client.tcp_sender.send(
                        TcpPacket::UserJoined{
                            uid,
                            username: username.clone(),
                        }
                    );
            }else{
                let _ = client.tcp_sender.send(TcpPacket::SendUID{uid});
            }
        }
    }



    /*
        NORMAL COMMAND LOOP
    */

    loop {

        let mut command = [0u8; 1];


        if stream.read_exact(&mut command).is_err() {
            break;
        }


        match command[0] {


            // START STREAM
            0x01 => {


                let mut state =
                    state.write().unwrap();

                if state.streams.contains_key(&uid){
                    continue;
                }

                state.streams.insert(
                    uid,
                    Stream {
                        latest_packet: None,
                    }
                );

                for client in state.users.values()
                {
                    let _ =
                        client.tcp_sender.send(
                            TcpPacket::StreamStarted{
                                uid,
                                username: username.clone(),
                            }
                        );
                }
            }


            // STOP STREAM
            0x02 => {

                let mut state =
                    state.write().unwrap();

                if !state.streams.contains_key(&uid){
                    continue;
                }

                state.streams.remove(&uid);
                for client in state.users.values()
                {
                    let _ =
                        client.tcp_sender.send(
                            TcpPacket::StreamStopped{
                                uid,
                                username: username.clone(),
                            }
                        );
                }
            }


            // DISCONNECT
            0x03 => {

                let mut state =
                    state.write().unwrap();


                state.users.remove(&uid);
                state.streams.remove(&uid);


                for client in state.users.values()
                {
                    let _ =
                        client.tcp_sender.send(
                            TcpPacket::UserLeft{
                                uid,
                                username: username.clone(),
                                
                             }
                        );
                }

                break;
            }


            _ => {}
        }
    }



    /*
        CLEANUP
    */

    let mut state =
        state.write().unwrap();


    state.users.remove(&uid);


    for client in state.users.values()
    {
        let _ =
            client.tcp_sender.send(
                TcpPacket::UserLeft{
                    uid, 
                    username: username.clone(),
                }
            );
    }


    Ok(())
}

                
