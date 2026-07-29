use std::{
    io::{Result, prelude::*, stdin}, net::TcpStream, sync::{Arc, RwLock, atomic::Ordering}, thread,
};

use crate::client::{state::ClientState, ui::state::AppState};

pub fn handle_tcp(
    mut tcp: TcpStream,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
) -> Result<()> {
    println!("test");
    let mut receive_stream = tcp.try_clone()?;

    thread::spawn(move || {
        let mut len_buf = [0u8; 4];

        loop {

            if let Err(e) = receive_stream.read_exact(&mut len_buf) {
                println!("Connection closed: {e}");
                break;
            }

            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buffer = vec![0u8; len];

            if let Err(e) = receive_stream.read_exact(&mut buffer) {
                println!("Connection closed: {e}");
                break;
            }

            println!("{}", String::from_utf8_lossy(&buffer));
        }
    });


    loop {

        println!();
        println!("1 - start streaming");
        println!("2 - stop streaming");
        println!("3 - disconnect");


        let mut input = String::new();

        stdin()
            .read_line(&mut input)?;


        match input.trim() {

            "1" => {
                // START_STREAM
                tcp.write_all(&[0x01])?;
                //let client_state = client_state.write().unwrap();
                client_state.streaming.store(true, Ordering::Relaxed);
                println!("Streaming started");
            }


            "2" => {
                // STOP_STREAM
                tcp.write_all(&[0x02])?;
                //let client_state = client_state.write().unwrap();
                client_state.streaming.store(false, Ordering::Relaxed);
                println!("Streaming stopped");
            }


            "3" => {
                // DISCONNECT
                tcp.write_all(&[0x03])?;

                break;
            }


            _ => {
                println!("Invalid command");
            }
        }
    }
    Ok(())
}


pub fn start_receiver(
    mut tcp: TcpStream,
    app_state: Arc<RwLock<AppState>>,
) -> Result<()> {
    thread::spawn(move || {
        loop {
            match run_receiver(&mut tcp, &app_state){
                Ok(true) => {}
                Ok(false) => {break},
                Err(e)  => {
                    eprintln!("TCP receiver error: {e}");
                    break;
                }
            }

        }
    });

    Ok(())
}

fn run_receiver(
    tcp: &mut TcpStream,
    app_state: &Arc<RwLock<AppState>>,
) -> Result<bool> {
    let mut len_buf = [0u8; 4];

    if let Err(e) = tcp.read_exact(&mut len_buf) {
        println!("Connection closed: {e}");
        return Err(e);
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buffer = vec![0u8; len];

    if let Err(e) = tcp.read_exact(&mut buffer) {
        println!("Connection closed: {e}");
        return Err(e);
    }

    println!("{}", String::from_utf8_lossy(&buffer));


    Ok(true)

}

pub fn start_sender(
    mut tcp: TcpStream,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
) -> Result<()> {
    thread::spawn(move || {

        loop {
            match run_sender(&mut tcp, &client_state, &app_state){
                Ok(true) => {}
                Ok(false) => {break},
                Err(e) => {
                    eprintln!("TCP sender error: {e}");
                    break;
                }
            };
        }

    });

    Ok(())
}


fn run_sender(
    tcp: &mut TcpStream,
    client_state: &Arc<ClientState>,
    app_state: &Arc<RwLock<AppState>>,
) -> Result<bool> {

        println!();
        println!("1 - start streaming");
        println!("2 - stop streaming");
        println!("3 - disconnect");


        let mut input = String::new();

        stdin()
            .read_line(&mut input)?;


        match input.trim() {

            "1" => {
                // START_STREAM
                tcp.write_all(&[0x01])?;
                //let client_state = client_state.write().unwrap();
                client_state.streaming.store(true, Ordering::Relaxed);
                println!("Streaming started");

                Ok(true)
            }


            "2" => {
                // STOP_STREAM
                tcp.write_all(&[0x02])?;
                //let client_state = client_state.write().unwrap();
                client_state.streaming.store(false, Ordering::Relaxed);
                println!("Streaming stopped");

                Ok(true)
            }


            "3" => {
                // DISCONNECT
                tcp.write_all(&[0x03])?;

                Ok(false)
            }


            _ => {
                println!("Invalid command");

                Ok(true)
            }
        }
}