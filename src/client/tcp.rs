use std::{
    io::{Result, prelude::*, stdin}, net::TcpStream, sync::{Arc, RwLock, atomic::Ordering}, thread,
};

use crate::client::ClientState;

pub fn handle_tcp(
    mut stream: TcpStream,
    state: Arc<RwLock<ClientState>>,
) -> Result<()> {
    println!("test");
    let mut receive_stream = stream.try_clone()?;

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
                stream.write_all(&[0x01])?;
                let state = state.write().unwrap();
                state.streaming.store(true, Ordering::Relaxed);
                println!("Streaming started");
            }


            "2" => {
                // STOP_STREAM
                stream.write_all(&[0x02])?;
                let state = state.write().unwrap();
                state.streaming.store(false, Ordering::Relaxed);
                println!("Streaming stopped");
            }


            "3" => {
                // DISCONNECT
                stream.write_all(&[0x03])?;

                break;
            }


            _ => {
                println!("Invalid command");
            }
        }
    }
    Ok(())
}
