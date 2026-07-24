use crate::server::auth::{generate_challenge, create_response, SERVER_KEY};
use crate::server::state::{ServerState, TcpMessage, Client};

use std::{
    net::{TcpStream},
    io::{prelude::*,Result},
    sync::{Mutex, Arc},
    thread,
    sync::mpsc::{channel},
};


pub fn handle_tcp(
    mut stream: TcpStream,
    state: Arc<Mutex<ServerState>>,
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
        stream.write_all(&[0x00])?; // denied
        println!("{username} failed authentication");
        return Ok(());
    }


    stream.write_all(&[0x01])?; // accepted
    println!("{username} authenticated");


    /*
        CREATE MPSC
    */

    let (tx, rx) =
        channel::<TcpMessage>();


    /*
        WRITER THREAD
    */

    let mut write_stream =
        stream.try_clone()?;


    thread::spawn(move || {

        let mut message_to_send;
        let mut message_bytes;

        while let Ok(message) = rx.recv() {

            match message {

                TcpMessage::Authenticated => {
                    let _ =
                        write_stream.write_all(&[0x10]);
                }


                TcpMessage::UserJoined(name) => {
                    message_to_send = format!("{name} joined");
                    message_bytes = message_to_send.as_bytes();

                    println!("{message_to_send}");

                    if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }

                    if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }
                }


                TcpMessage::UserLeft(name) => {
                    message_to_send = format!("{name} left");
                    message_bytes = message_to_send.as_bytes();

                    println!("{message_to_send}");

                    if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }

                    if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }
                }


                TcpMessage::UserStarted(name) => {
                    message_to_send = format!("{name} started streaming");
                    message_bytes = message_to_send.as_bytes();
                    
                    println!("{message_to_send}");

                    if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }

                    if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }
                }


                TcpMessage::UserStopped(name) => {
                    message_to_send = format!("{name} stopped streaming");
                    message_bytes = message_to_send.as_bytes();

                    println!("{message_to_send}");

                    if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }

                    if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                        println!("Connection closed: {e}");
                        break;
                    }
                }


                TcpMessage::Error(err) => {
                    println!("error: {}", err);
                }
            }
        }
    });



    /*
        REGISTER CLIENT
    */

    {
        let mut state =
            state.lock().unwrap();


        state.users.insert(
            username.clone(),
            Client {
                username: username.clone(),
                tcp_sender: tx.clone(),
                udp_addr: None,
                streaming: false,
            }
        );


        // notify existing users
        for (name, client)
            in &state.users
        {
            if name != &username {
                let _ =
                    client.tcp_sender.send(
                        TcpMessage::UserJoined(
                            username.clone()
                        )
                    );
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
                    state.lock().unwrap();


                if let Some(client)
                    = state.users.get_mut(&username)
                {
                    if client.streaming == true{
                        continue;
                    }
                    client.streaming = true;
                }


                for client in state.users.values()
                {
                    let _ =
                        client.tcp_sender.send(
                            TcpMessage::UserStarted(
                                username.clone()
                            )
                        );
                }
            }


            // STOP STREAM
            0x02 => {

                let mut state =
                    state.lock().unwrap();


                if let Some(client)
                    = state.users.get_mut(&username)
                {
                    if client.streaming == false{
                        continue;
                    }
                    client.streaming = false;
                }


                for client in state.users.values()
                {
                    let _ =
                        client.tcp_sender.send(
                            TcpMessage::UserStopped(
                                username.clone()
                            )
                        );
                }
            }


            // DISCONNECT
            0x03 => {

                let mut state =
                    state.lock().unwrap();


                state.users.remove(&username);


                for client in state.users.values()
                {
                    let _ =
                        client.tcp_sender.send(
                            TcpMessage::UserLeft(
                                username.clone()
                            )
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
        state.lock().unwrap();


    state.users.remove(&username);


    for client in state.users.values()
    {
        let _ =
            client.tcp_sender.send(
                TcpMessage::UserLeft(
                    username.clone()
                )
            );
    }


    Ok(())
}

                
