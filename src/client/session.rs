use std::sync::{Arc};

use tokio::{stream, sync::mpsc};
use quinn::{RecvStream, SendStream};


use crate::{client::{command::{ClientCommand, ClientEvent, EncoderCommand}, endpoint}, media::{decoder::VideoDecoder, encoder::EncodedFrame, frame::RawFrame, reassembler::VideoReassembler, state::Media}, network::{datagram::send_frame, stream::{receive_packet, send_packet}}, protocol::{command::{ClientPacket, ServerPacket}, video::VideoPacket}};

use super::config::ClientConfig;


pub struct ClientSession{
    pub connection: quinn::Connection,
    // pub command_tx: tokio::sync::mpsc::Sender<ClientPacket>,
    // pub video_tx: mpsc::Sender<VideoPacket>,
}

impl ClientSession {

    pub async fn connect(
        config: &ClientConfig,
        media: Arc<crate::media::state::Media>,
        command_rx: mpsc::Receiver<ClientCommand>,
        event_tx: mpsc::Sender<ClientEvent>,
        my_video_rx: mpsc::Receiver<EncodedFrame>,
        others_video_tx: mpsc::Sender<RawFrame>,
        encoder_tx: mpsc::Sender<EncoderCommand>,
    ) -> anyhow::Result<(Self, u64)> {
        let server_addr = config.server_addr()?;

        let endpoint =
            endpoint::make_client_endpoint(
                server_addr,
                config.fingerprint.clone(),
            )?;

        println!(
            "Trying to connect to: {} {}",
            server_addr,
            config.server_name
        );

        let connection =
            endpoint
                .connect(
                    server_addr,
                    &config.server_name,
                )?
                .await?;

        println!(
            "Connected securely to {}",
            connection.remote_address()
        );

        let (mut send, mut recv) =
            connection.open_bi().await?;

        // Authenticate BEFORE spawning the normal
        // control tasks.
        let uid =
            login(
                &config,
                &mut send,
                &mut recv,
            ).await?;

        println!("Authed with UID {uid}");

        Self::spawn_sender(media.clone(), send, command_rx);

        Self::spawn_receiver(uid, recv, event_tx.clone(), encoder_tx);

        Self::spawn_video_sender(
            connection.clone(),
            uid,
            my_video_rx,
        );

        Self::spawn_video_receiver(
            connection.clone(),
            others_video_tx,
        );


        Ok((Self {
            connection,
            //command_tx,
        }, uid))
    }

    fn spawn_sender(
        media: Arc<crate::media::state::Media>,
        mut send: SendStream,
        mut command_rx: mpsc::Receiver<ClientCommand>,
    ) {
        tokio::spawn(async move {

            while let Some(command) =
                command_rx.recv().await
            {
                let packet = match command {

                    ClientCommand::StartStream => {
                        media.start_streaming();

                        ClientPacket::StreamStarted
                    },

                    ClientCommand::StopStream => {
                        media.stop_streaming();

                        ClientPacket::StreamStopped
                    }

                    ClientCommand::WatchStream { uid } =>
                        ClientPacket::WatchStream { uid },

                    ClientCommand::LeaveStream { uid } =>
                        ClientPacket::LeaveStream { uid },

                    ClientCommand::Disconnect => ClientPacket::Disconnect,
                };

                if let Err(e) =
                    send_packet(&mut send, &packet).await
                {
                    eprintln!(
                        "control send error: {e}"
                    );

                    break;
                }
            }
        });
    }


    fn spawn_receiver(
        my_uid: u64,
        mut recv: RecvStream,
        event_tx: mpsc::Sender<ClientEvent>,
        encoder_tx: mpsc::Sender<EncoderCommand>
    ) {
        tokio::spawn(async move {

            loop {
                let packet = match receive_packet::<ServerPacket>(&mut recv).await {
                    Ok(packet) => packet,
                    Err(e) => {
                        eprintln!("control receive error: {e}");
                        break;
                    }
                };

                match packet {
                    ServerPacket::UserJoined {
                        uid,
                        username,
                    } => {
                        let _ = event_tx
                            .send(ClientEvent::UserJoined { uid, username })
                            .await;
                    }

                    ServerPacket::UserLeft {
                        uid,
                        username,
                    } => {
                        let _ = event_tx
                            .send(ClientEvent::UserLeft { uid, username })
                            .await;
                    }

                    ServerPacket::StreamStarted {
                        uid,
                        username,
                    } => {
                        let _ = event_tx
                            .send(ClientEvent::StreamStarted { uid, username })
                            .await;
                    }

                    ServerPacket::StreamStopped {
                        uid,
                        username,
                    } => {
                        let _ = event_tx
                            .send(ClientEvent::StreamStopped { uid, username })
                            .await;
                    }

                    ServerPacket::InitialState { users, streams } => {
                        let _ = event_tx
                            .send(ClientEvent::InitialState { users, streams })
                            .await;
                    }

                    ServerPacket::Error { error } => {
                        let _ = event_tx
                            .send(ClientEvent::Error(error))
                            .await;
                    }

                    ServerPacket::WatchStream {
                        uid,
                        stream_uid,
                    } => {
                        println!(
                            "watching stream {}",
                            stream_uid,
                        );

                        if my_uid == stream_uid{
                            println!("someone wants to watch me >.< ");
                            match encoder_tx
                                .send(EncoderCommand::ForceKeyframe)
                                .await{
                                    Ok(()) => print!("Forcekeyframe sent"),
                                    Err(e) => println!("FAILED to send forcekeyframe: {e}"),
                                }
                        }


                        let _ = event_tx
                            .send(ClientEvent::WatchStream {
                                uid,
                                stream_uid,
                            })
                            .await;
                    }

                    ServerPacket::AuthAccepted { .. }
                    | ServerPacket::AuthDenied => {
                        eprintln!("unexpected authentication packet");
                    }
                }
            }
        });
    }

    fn spawn_video_sender(
        connection: quinn::Connection,
        uid: u64,
        mut video_rx: mpsc::Receiver<EncodedFrame>,
    ) {
        tokio::spawn(async move {
            while let Some(frame) = video_rx.recv().await {
                let packets = crate::media::encoder::packetize(uid, &frame);

                for packet in packets {
                    let data = packet.encode();

                    match connection.send_datagram(data.into()) {
                        Ok(()) => {}

                        Err(quinn::SendDatagramError::UnsupportedByPeer) => {
                            eprintln!("peer does not support QUIC datagrams");
                            return;
                        }

                        Err(quinn::SendDatagramError::Disabled) => {
                            eprintln!("QUIC datagrams are disabled");
                            return;
                        }

                        Err(quinn::SendDatagramError::TooLarge) => {
                            eprintln!("video datagram too large");
                            continue;
                        }

                        Err(quinn::SendDatagramError::ConnectionLost(e)) => {
                            eprintln!("connection lost while sending video: {e}");
                            return;
                        }
                    }
                }
            }
        });
    }

    
    fn spawn_video_receiver(
        connection: quinn::Connection,
        video_tx: mpsc::Sender<RawFrame>,
    ) {
        tokio::spawn(async move {
            let mut reassembler = VideoReassembler::new();

            let mut decoder = match VideoDecoder::new() {
                Ok(decoder) => decoder,
                Err(e) => {
                    eprintln!("failed to initialize H264 decoder: {e}");
                    return;
                }
            };

            loop {
                let data = match connection.read_datagram().await {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("video receive error: {e}");
                        break;
                    }
                };

                // println!("received datagram: {} bytes", data.len());

                let Some(packet) = VideoPacket::decode(&data) else {
                    eprintln!("invalid video packet");
                    continue;
                };

                // println!(
                //     "packet frame={} {}/{}",
                //     packet.frame_id,
                //     packet.packet_index + 1,
                //     packet.packet_total
                // );

                if let Some(frame) = reassembler.push(packet) {
                    // println!(
                    //     "REASSEMBLED frame {} ({} bytes)",
                    //     frame.sequence,
                    //     frame.data.len()
                    // );

                    match decoder.decode_frame(&frame) {
                        Ok(Some(raw_frame)) => {
                            // println!(
                            //     "decoded frame {} ({}x{})",
                            //     raw_frame.sequence,
                            //     raw_frame.width,
                            //     raw_frame.height
                            // );

                            if video_tx.send(raw_frame).await.is_err() {
                                eprintln!("video renderer channel closed");
                                break;
                            }
                        }

                        Ok(None) => {
                            println!("decoder returned None");
                        }

                        Err(e) => {
                            eprintln!("H264 decode error: {e}");
                        }
                    }
                }
            }
        });
    }
}

async fn login(
    config: &ClientConfig,
    send: &mut SendStream,
    recv: &mut RecvStream,
) -> anyhow::Result<u64> {

    let packet =
        ClientPacket::Authenticate {
            username: config.username.clone(),
            password: config.server_password.clone(),
        };

    send_packet(send, &packet).await?;

    println!("login sent");

    let packet =
        receive_packet::<ServerPacket>(recv).await?;

    println!("auth received");

    match packet {

        ServerPacket::AuthAccepted { uid } => {
            Ok(uid)
        }

        ServerPacket::AuthDenied => {
            Err(anyhow::anyhow!("Auth denied"))
        }

        _ => {
            Err(anyhow::anyhow!(
                "unexpected packet during authentication"
            ))
        }
    }
}