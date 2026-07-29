use std::{
    io::{Result, prelude::*}, net::{SocketAddr, UdpSocket}, sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}}, thread, time::Duration,
};

use crate::{client::{state::ClientState, ui::state::AppState}, media::state::MediaState, protocol::{udp::UdpPacket, video::VideoPacket}};


pub fn udp_loop(
    udp: UdpSocket,
    udp_addr: SocketAddr,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,

)-> Result<()> {

    let recv_socket = udp.try_clone()?;
    let send_socket = udp.try_clone()?;

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

        //let client_state = client_state.read().unwrap();

        if client_state.streaming.load(Ordering::Relaxed) {

            let packet = UdpPacket::Video(
                VideoPacket {
                uid: client_state.uid.load(Ordering::Acquire),
                sequence: client_state.sequence.load(Ordering::Acquire),
                data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            });


            let bytes = packet.encode();



            send_socket.send_to(&bytes, udp_addr)?;
            println!("sent");
        
        }


    }


    Ok(())
}


pub fn start_receiver (
    mut udp: UdpSocket,
    udp_addr: SocketAddr,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,
)-> Result<()> {
    thread::spawn(move || {

        loop {
            match run_receiver(&mut udp, udp_addr, &client_state, &app_state, &media_state){
                Ok(true) => {}
                Ok(false) => {break},
                Err(e) => {
                    eprintln!("UDP receiver error: {e}");
                    break;
                }
            };
        }

    });

    Ok(())
}

fn run_receiver(
    udp: &mut UdpSocket,
    udp_addr: SocketAddr,
    client_state: &Arc<ClientState>,
    app_state: &Arc<RwLock<AppState>>,
    media_state: &Arc<MediaState>,
) -> Result<bool> {
        let mut buf   = [0; 1024];

        let (len, addr) = udp.recv_from(&mut buf)?;

            let Some(packet) = UdpPacket::decode(&buf[..len]) else {
                return Ok(true);
            };

            match packet {
                UdpPacket::Register { .. } => {}

                UdpPacket::Video(video) => {
                    println!("{:?}", video.data);
                }

                UdpPacket::Heartbeat => {}
            }

            Ok(true)
}


pub fn start_sender (
    mut udp: UdpSocket,
    udp_addr: SocketAddr,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,
)-> Result<()> {
    thread::spawn(move || {

        loop {
            match run_sender(&mut udp, udp_addr, &client_state, &app_state, &media_state){
                Ok(true) => {}
                Ok(false) => {break},
                Err(e) => {
                    eprintln!("UDP sender error: {e}");
                    break;
                }
            };

            thread::sleep(Duration::from_millis(16));
        }

    });

    Ok(())
}

fn run_sender(
    udp: &mut UdpSocket,
    udp_addr: SocketAddr,
    client_state: &Arc<ClientState>,
    app_state: &Arc<RwLock<AppState>>,
    media_state: &Arc<MediaState>,
) -> Result<bool> {

//let client_state = client_state.read().unwrap();

        if client_state.streaming.load(Ordering::Relaxed) {

            let packet = UdpPacket::Video(
                VideoPacket {
                uid: client_state.uid.load(Ordering::Acquire),
                sequence: client_state.sequence.load(Ordering::Acquire),
                data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            });


            let bytes = packet.encode();



            udp.send_to(&bytes, udp_addr)?;
            println!("sent");
        
        }

        Ok(true)
        
}