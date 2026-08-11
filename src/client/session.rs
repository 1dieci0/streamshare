use tokio::sync::mpsc;
use quinn::{RecvStream, SendStream};

use anyhow::anyhow;

use crate::{client::{command::{ClientCommand, ClientEvent}, endpoint}, network::stream::{receive_packet, send_packet}, protocol::command::{ClientPacket, ServerPacket}};

use super::config::ClientConfig;


pub struct ClientSession{
    pub connection: quinn::Connection,
    // pub command_tx: tokio::sync::mpsc::Sender<ClientPacket>,
    // pub video_tx: mpsc::Sender<VideoPacket>,
}

impl ClientSession {

    pub async fn connect(
        config: &ClientConfig,
        command_rx: mpsc::Receiver<ClientCommand>,
        event_tx: mpsc::Sender<ClientEvent>,
    ) -> anyhow::Result<Self> {
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

        //let (command_tx, command_rx) = mpsc::channel::<ClientPacket>(64);

        Self::spawn_sender(send, command_rx);

        Self::spawn_receiver(recv, event_tx.clone());

        Ok(Self {
            connection,
            //command_tx,
        })
    }

    fn spawn_sender(
        mut send: SendStream,
        mut command_rx: mpsc::Receiver<ClientCommand>,
    ) {
        tokio::spawn(async move {

            while let Some(command) =
                command_rx.recv().await
            {
                let packet = match command {

                    ClientCommand::StartStream =>
                        ClientPacket::StreamStarted,

                    ClientCommand::StopStream =>
                        ClientPacket::StreamStopped,

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
        mut recv: RecvStream,
        event_tx: mpsc::Sender<ClientEvent>,
    ) {
        tokio::spawn(async move {

            loop {
                match receive_packet::<ServerPacket>(
                    &mut recv
                ).await {

                    Ok(packet) => {

                        let event =
                            match packet {

                                ServerPacket::UserJoined {
                                    uid,
                                    username,
                                } =>
                                    ClientEvent::UserJoined {
                                        uid,
                                        username,
                                    },

                                ServerPacket::UserLeft {
                                    uid,
                                    username,
                                } =>
                                    ClientEvent::UserLeft {
                                        uid,
                                        username,
                                    },

                                ServerPacket::StreamStarted {
                                    uid,
                                    username,
                                } =>
                                    ClientEvent::StreamStarted {
                                        uid,
                                        username,
                                    },

                                ServerPacket::StreamStopped {
                                    uid,
                                    username,
                                } =>
                                    ClientEvent::StreamStopped {
                                        uid,
                                        username,
                                    },

                                ServerPacket::InitialState { users, streams } => {
                                    ClientEvent::InitialState { users, streams }
                                },

                                ServerPacket::Error { error } =>
                                    ClientEvent::Error(error),

                                ServerPacket::AuthAccepted { .. }
                                | ServerPacket::AuthDenied => {
                                    eprintln!(
                                        "unexpected authentication packet"
                                    );

                                    continue;
                                }
                            };

                        if event_tx.send(event).await.is_err() {
                            break;
                        }
                    }

                    Err(e) => {
                        eprintln!(
                            "control receive error: {e}"
                        );

                        break;
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
