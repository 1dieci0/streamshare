use std::{
    io::{Result, prelude::*, stdin}, net::TcpStream, sync::{Arc, RwLock, atomic::Ordering}, thread,
};

use winit::event_loop::EventLoopProxy;

use crate::{client::{state::{ClientCommand, ClientState}, ui::{event::AppEvent, state::AppState}}, media::state::MediaState, protocol::tcp::TcpPacket};

pub fn start_receiver(
    mut tcp: TcpStream,
    media_state: Arc<MediaState>,
    app_state: Arc<RwLock<AppState>>,
    proxy: EventLoopProxy<AppEvent>,
) -> Result<()> {
    thread::spawn(move || {
        loop {
            match run_receiver(&mut tcp, &media_state, &app_state, &proxy){
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
    media_state: &Arc<MediaState>,
    app_state: &Arc<RwLock<AppState>>,
    proxy: &EventLoopProxy<AppEvent>,
) -> Result<bool> {
    let mut len_buf = [0u8; 4];
    tcp.read_exact(&mut len_buf)?;

    let len = u32::from_be_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    tcp.read_exact(&mut buf)?;




    let Some(packet) = TcpPacket::decode(&buf) else {
        println!("tcp packet receive error");
        return Ok(true);
    };

    match packet {
        TcpPacket::UserJoined { uid, username } => {
            app_state.write().unwrap().notifications.push_back(format!("{username} joined"));
            println!("{:?}", app_state.read().unwrap().notifications);
            let _ = proxy.send_event(AppEvent::UserJoined(uid));
        }

        TcpPacket::UserLeft { uid, username } => {
            app_state.write().unwrap().notifications.push_back(format!("{username} left"));
            let _ = proxy.send_event(AppEvent::UserLeft(uid));
        }

        TcpPacket::StreamStarted { uid, username } => {
            app_state.write().unwrap().notifications.push_back(format!("{username} started streaming"));
            app_state.write().unwrap().available_streams.insert(uid, super::ui::state::StreamInfo { uid, username, resolution: (1980, 1020), fps: (60) });
            let _ = proxy.send_event(AppEvent::StreamStarted(uid));
        }

        TcpPacket::StreamStopped { uid, username } => {
            app_state.write().unwrap().notifications.push_back(format!("{username} stopped streaming"));
            app_state.write().unwrap().available_streams.remove(&uid);
            let _ = proxy.send_event(AppEvent::StreamStopped(uid));
        }

        TcpPacket::Error { error } => {
            eprintln!("{error}");
        }

        _ => {}
    }


    Ok(true)

}

pub fn start_sender(
    mut tcp: TcpStream,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    proxy: EventLoopProxy<AppEvent>,
) -> Result<()> {
    thread::spawn(move || {

        loop {
            match run_sender(&mut tcp, &client_state, &app_state, &proxy){
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
    proxy: &EventLoopProxy<AppEvent>,
) -> Result<bool> {

        let command = {
            let mut cmd = client_state.command.lock().unwrap();

            let c = std::mem::replace(
                &mut *cmd,
                ClientCommand::None
            );

            c
        };


        match command {

            ClientCommand::StartStream => {
                // START_STREAM
                tcp.write_all(&[0x01])?;
                client_state.streaming.store(true, Ordering::Relaxed);
                println!("Streaming started");

                Ok(true)
            }


            ClientCommand::StopStream => {
                // STOP_STREAM
                tcp.write_all(&[0x02])?;
                client_state.streaming.store(false, Ordering::Relaxed);
                println!("Streaming stopped");

                Ok(true)
            }


            ClientCommand::Disconnect => {
                // DISCONNECT
                tcp.write_all(&[0x03])?;
                println!("Disconnect");

                Ok(false)
            }


            _ => {
                //println!("Invalid command");

                Ok(true)
            }
        }
}