use std::{
    collections::HashMap, net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket}, path::Path, sync::{Arc, atomic::{AtomicU64, Ordering}}, thread,
};

mod endpoint;
mod config;

use anyhow::anyhow;
use quinn::{ClientConfig, SendStream, RecvStream};
use winit::event;

use crate::{network::{self, stream::{receive_packet, send_packet}}, protocol::command::{ClientPacket, ServerPacket}};

use tokio::sync::mpsc;
use tokio::sync::RwLock;

struct ClientConnection {
    uid: u64,
    username: String,
    event_tx: mpsc::Sender<ServerPacket>,
}

pub struct ServerState {
    pub clients: HashMap<u64, ClientConnection>,
    pub next_uid: AtomicU64,
}

pub struct Server{
    pub config: config::ServerConfig,
    pub state: Arc<RwLock<ServerState>>,
}


impl Server{
    pub fn new(config_path: &str) -> anyhow::Result<Self>{
        let config =
            config::ServerConfig::load_or_create(
                config_path
        )?;

        Ok(Self{
            config,
            state: Arc::new(RwLock::new(ServerState{
                clients: HashMap::new(),
                next_uid: AtomicU64::new(1),
            }))
        })
    }

    pub async fn start(self) -> anyhow::Result<()> {

        let endpoint = endpoint::make_server_endpoint(
            self.config.clone()
        )?;

        let server = Arc::new(self);

        while let Some(connecting) = endpoint.accept().await {

            let server = Arc::clone(&server);

            println!("incoming connection");


            tokio::spawn(async move {

                match connecting.await {

                    Ok(connection) => {
                        println!(
                            "Client connected: {}",
                            connection.remote_address()
                        );
                        
                        if let Err(e) = server.handle_connection(connection).await
                        {
                            eprintln!(
                                "connection error: {e}"
                            );
                        }
                    }


                    Err(e) => {
                        eprintln!(
                            "failed connection: {e}"
                        );
                    }
                }

            });
        }

        Ok(())
    }

    async fn handle_connection(
        &self,
        connection: quinn::Connection,
    ) -> anyhow::Result<()> {

        let (mut send, mut recv) =
                connection.accept_bi().await?;
            
        let (uid, username) = self.login(&mut send, &mut recv).await?;

        println!("client authed");

        let (event_tx, mut event_rx) = mpsc::channel::<ServerPacket>(128);

        {
            let mut state = self.state.write().await;

            state.clients.insert(
                uid,
                ClientConnection{
                    uid,
                    username: username.clone(),
                    event_tx: event_tx.clone(),
                }
            );
        }

        self.broadcast(
            ServerPacket::UserJoined { uid, username: username.clone() }, 
            Some(uid)
        ).await;



        let send_task = tokio::spawn(async move {
            while let Some(packet) = event_rx.recv().await {
                if let Err(e) = send_packet(&mut send, &packet).await {
                    eprintln!("send error: {e}");
                    break;
                }
            }
        });



        loop {
            let packet = match receive_packet::<ClientPacket>(&mut recv).await {
                Ok(packet) => packet,

                Err(e) => {
                    println!("client {uid} disconnected: {e}");
                    break;
                }
            };

            self.handle_command(
                uid,
                username.clone(),
                packet,
            ).await?;
        }

        {
            let mut state = self.state.write().await;
            state.clients.remove(&uid);
        }

        // Notify everyone else.
        self.broadcast(
            ServerPacket::UserLeft {
                uid,
                username,
            },
            Some(uid),
        )
        .await;

        // Stop sender task.
        send_task.abort();

        Ok(())

    }



    async fn login(
        &self,
        send: &mut SendStream,
        recv: &mut RecvStream
    ) -> anyhow::Result<(u64, String)> {
        let packet: ClientPacket = receive_packet(recv).await?;

        match packet{
            ClientPacket::Authenticate { username, password } => {
                if password == self.config.security.password{

                    let uid = 0;

                    let uid = self.state.read().await.next_uid.fetch_add(1, Ordering::Relaxed);

                    println!("about to send uid");

                    send_packet(send, &ServerPacket::AuthAccepted{uid}
                    ).await?;

                    println!("uid sent");

                    Ok((uid, username))

                }else{
                    send_packet(send, &ServerPacket::AuthDenied).await?;

                    Err(anyhow::anyhow!("Auth failed"))
                }
            }

            _ => {Err(anyhow::anyhow!("No auth yet"))}
        }
    }


    async fn handle_command(
        &self,
        uid: u64,
        username: String,
        packet: ClientPacket,
    ) -> anyhow::Result<()> {
        match packet {
            ClientPacket::StreamStarted => {
                println!("{uid} -> StartStream");

                // TODO: tell media subsystem to start
                // TODO: later create/use QUIC datagram path

                self.broadcast(
                    ServerPacket::StreamStarted { uid, username },
                    None,
                ).await;
            }

            ClientPacket::StreamStopped => {
                println!("{uid} -> StopStream");

                // TODO: tell media subsystem to stop

                self.broadcast(
                    ServerPacket::StreamStopped { uid, username },
                    None,
                ).await;
            }

            ClientPacket::Disconnect => {
                return Err(anyhow::anyhow!("user wants to disconnect"))
            }

            ClientPacket::WatchStream { uid } => {

            }

            ClientPacket::LeaveStream { uid } => {

            }

            ClientPacket::Authenticate { .. } => {
                println!("what is this guy doing lol {uid} {username}");
            }
        }

        Ok(())
    }



    async fn broadcast(
        &self,
        packet: ServerPacket,
        except: Option<u64>,
    ) {
        let senders = {
            let state = self.state.read().await;

            state
                .clients
                .iter()
                .filter_map(|(&uid, client)| {
                    if Some(uid) == except {
                        None
                    } else {
                        Some(client.event_tx.clone())
                    }
                })
                .collect::<Vec<_>>()
        };

        for tx in senders {
            let _ = tx.send(packet.clone()).await;
        }
    }

}

