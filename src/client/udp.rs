use std::{
    collections::HashMap,
    io::Result,
    net::{SocketAddr, UdpSocket},
    sync::{atomic::Ordering, Arc, RwLock},
    thread,
    time::Duration,
};

use winit::event_loop::EventLoopProxy;

use crate::{
    client::{state::ClientState, ui::{event::AppEvent, state::AppState}},
    media::{decoder::VideoDecoder, encoder::VideoEncoder, state::MediaState},
    protocol::udp::UdpPacket,
};

struct FrameBuffer {
    codec: u8,
    width: usize,
    height: usize,
    packets: Vec<Option<Vec<u8>>>,
    received: usize,
}

pub fn start_receiver(
    mut udp: UdpSocket,
    _udp_addr: SocketAddr,
    client_state: Arc<ClientState>,
    _app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,
    proxy: EventLoopProxy<AppEvent>,
) -> Result<()> {
    thread::spawn(move || {
        let mut frames: HashMap<(u64, u64), FrameBuffer> = HashMap::new();
        let mut decoder = match VideoDecoder::new() {
            Ok(decoder) => decoder,
            Err(e) => {
                eprintln!("Failed to create video decoder: {e}");
                return;
            }
        };

        loop {
            match run_receiver(
                &mut udp,
                &client_state,
                &media_state,
                &proxy,
                &mut frames,
                &mut decoder,
            ) {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => {
                    eprintln!("UDP receiver error: {e}");
                    break;
                }
            }
        }
    });

    Ok(())
}

fn run_receiver(
    udp: &mut UdpSocket,
    client_state: &Arc<ClientState>,
    media_state: &Arc<MediaState>,
    proxy: &EventLoopProxy<AppEvent>,
    frames: &mut HashMap<(u64, u64), FrameBuffer>,
    decoder: &mut VideoDecoder,
) -> Result<bool> {
    let mut buf = [0u8; 1500];
    let (len, _) = udp.recv_from(&mut buf)?;

    let Some(packet) = UdpPacket::decode(&buf[..len]) else {
        println!("problema pacchetto");
        return Ok(true);
    };
    match packet {
        UdpPacket::Register { .. } => {}
        UdpPacket::Heartbeat => {}
        UdpPacket::Video(video) => {
            if video.uid == client_state.uid.load(Ordering::Acquire) {
                return Ok(true);
            }

            let key = (video.uid, video.frame_id);

            let should_reset = frames
                .get(&key)
                .map(|existing| existing.packets.len() != video.packet_total as usize)
                .unwrap_or(false);

            if should_reset {
                frames.remove(&key);
            }

            let entry = frames.entry(key).or_insert_with(|| FrameBuffer {
                codec: video.codec,
                width: video.width as usize,
                height: video.height as usize,
                packets: vec![None; video.packet_total as usize],
                received: 0,
            });

            entry.codec = video.codec;
            entry.width = video.width as usize;
            entry.height = video.height as usize;

            if video.packet_index as usize >= entry.packets.len() {
                return Ok(true);
            }

            if entry.packets[video.packet_index as usize].is_none() {
                entry.packets[video.packet_index as usize] = Some(video.data);
                entry.received += 1;
            }
            if entry.received == entry.packets.len() {
                let frame_buffer = frames.remove(&key).expect("frame buffer should exist");
                let mut encoded = Vec::new();

                for packet in frame_buffer.packets.into_iter().flatten() {
                    encoded.extend(packet);
                }

            let Some(frame) = decoder.decode_frame(
                frame_buffer.codec,
                video.frame_id,
                frame_buffer.width,
                frame_buffer.height,
                &encoded,
            )
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))? else {
                return Ok(true);
            };

                media_state.update_remote(video.uid, frame);
                let _ = proxy.send_event(AppEvent::NewFrame(video.uid));
            }
        }
    }

    Ok(true)
}

pub fn start_sender(
    mut udp: UdpSocket,
    udp_addr: SocketAddr,
    client_state: Arc<ClientState>,
    _app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,
) -> Result<()> {
    thread::spawn(move || {
        let mut encoder = match VideoEncoder::new() {
            Ok(encoder) => encoder,
            Err(e) => {
                eprintln!("Failed to create video encoder: {e}");
                return;
            }
        };

        loop {
            match run_sender(&mut udp, udp_addr, &client_state, &media_state, &mut encoder) {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => {
                    eprintln!("UDP sender error: {e}");
                    break;
                }
            }

            thread::sleep(Duration::from_millis(16));
        }
    });

    Ok(())
}

fn run_sender(
    udp: &mut UdpSocket,
    udp_addr: SocketAddr,
    client_state: &Arc<ClientState>,
    media_state: &Arc<MediaState>,
    encoder: &mut VideoEncoder,
) -> Result<bool> {
    if client_state.streaming.load(Ordering::Relaxed) {
        let Some(frame) = media_state.capture() else {
            return Ok(true);
        };

        let uid = client_state.uid.load(Ordering::Acquire);
        let frame_id = client_state.sequence.fetch_add(1, Ordering::AcqRel);
        let packets = encoder
            .encode_and_packetize(uid, frame_id, &frame)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        for packet in packets {
            let bytes = UdpPacket::Video(packet).encode();
            udp.send_to(&bytes, udp_addr)?;
        }
    }

    Ok(true)
}