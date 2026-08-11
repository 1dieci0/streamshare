use std::{
    collections::HashMap, net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket}, path::Path, sync::{Arc, atomic::{AtomicU64, Ordering}}, thread,
};
use bytes::Bytes;

mod endpoint;
mod config;

use anyhow::anyhow;
use quinn::{ClientConfig, SendStream, RecvStream};
use winit::event;

use crate::protocol::{info::{StreamInfo, UserInfo}, video::VideoPacket};
use crate::{network::{self, stream::{receive_packet, send_packet}}, protocol::command::{ClientPacket, ServerPacket}};

use tokio::sync::mpsc;
use tokio::sync::RwLock;

struct ClientConnection {
    uid: u64,
    username: String,
    event_tx: mpsc::Sender<ServerPacket>,
    connection: quinn::Connection,
    pub streaming: bool,

    pub watching: Option<u64>,
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
        self: Arc<Self>,
        connection: quinn::Connection,
    ) -> anyhow::Result<()> {

        let (mut send, mut recv) =
                connection.accept_bi().await?;
            
        let (uid, username) = self.login(&mut send, &mut recv).await?;

        println!("client {uid} authed");

        let (event_tx, mut event_rx) = mpsc::channel::<ServerPacket>(128);

        let snapshot = {
            let mut state = self.state.write().await;

            state.clients.insert(
                uid,
                ClientConnection{
                    uid,
                    username: username.clone(),
                    event_tx: event_tx.clone(),
                    connection: connection.clone(),
                    streaming: false,
                    watching: None,
                }
            );

            let users = state.clients
                .values()
                .map(|client| UserInfo {
                    uid: client.uid,
                    username: client.username.clone(),
                })
                .collect();

            let streams = state.clients
                .values()
                .filter(|client| client.streaming)
                .map(|client| StreamInfo {
                    uid: client.uid,
                    username: client.username.clone(),
                })
                .collect();

            (users, streams)

        };

        send_packet(&mut send, &ServerPacket::InitialState { users: snapshot.0, streams: snapshot.1 }).await?;
        

        

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

        let video_server = Arc::clone(&self);
        let video_connection = connection.clone();

        let datagram_task = tokio::spawn(async move {
            video_server
                .handle_video_datagrams(
                    video_connection,
                    uid
                )
                .await;
        });



        loop {
            let packet = match receive_packet::<ClientPacket>(&mut recv).await {
                Ok(packet) => packet,

                Err(e) => {
                    println!("client {uid} disconnected: {e}");
                    break;
                }
            };

            if let Err(e) =  self.handle_command(
                uid,
                username.clone(),
                packet,
            ).await {
                    println!("Error: {e}");
                    break;
            }
        }

        {
            let mut state = self.state.write().await;
            state.clients.remove(&uid);
            for viewer in state.clients.values_mut() {
                if viewer.watching == Some(uid) {
                    viewer.watching = None;
                }
            }
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
        datagram_task.abort();

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
                let mut state = self.state.write().await;

                let client = state.clients
                    .get_mut(&uid)
                    .ok_or_else(|| anyhow!("client not found"))?;

                client.streaming = true;

                drop(state);

                self.broadcast(
                    ServerPacket::StreamStarted { uid, username },
                    None,
                ).await;
            }

            ClientPacket::StreamStopped => {
                let mut state = self.state.write().await;

                let client = state.clients
                    .get_mut(&uid)
                    .ok_or_else(|| anyhow!("client not found"))?;

                client.streaming = false;

                // Anyone watching this stream must stop watching it.
                for viewer in state.clients.values_mut() {
                    if viewer.watching == Some(uid) {
                        viewer.watching = None;
                    }
                }

                drop(state);

                self.broadcast(
                    ServerPacket::StreamStopped { uid, username },
                    None,
                ).await;
            }

            ClientPacket::Disconnect => {
                return Err(anyhow::anyhow!("user {uid} {username} wants to disconnect"))
            }

            ClientPacket::WatchStream { uid: stream_uid } => {
                let mut state = self.state.write().await;

                let stream_exists = state.clients
                    .get(&stream_uid)
                    .map(|client| client.streaming)
                    .unwrap_or(false);

                if !stream_exists {
                    return Err(anyhow!(
                        "stream {stream_uid} does not exist"
                    ));
                }

                let viewer = state.clients
                    .get_mut(&uid)
                    .ok_or_else(|| anyhow!("viewer {uid} not found"))?;

                viewer.watching = Some(stream_uid);
            }

            ClientPacket::LeaveStream { uid: stream_uid } => {
                let mut state = self.state.write().await;

                let viewer = state.clients
                    .get_mut(&uid)
                    .ok_or_else(|| anyhow!("viewer {uid} not found"))?;

                if viewer.watching == Some(stream_uid) {
                    viewer.watching = None;
                }
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


    async fn handle_video_datagrams(
        &self,
        connection: quinn::Connection,
        source_uid: u64,
    ) {
        loop {
            let datagram = match connection.read_datagram().await {
                Ok(datagram) => datagram,

                Err(e) => {
                    eprintln!(
                        "video datagram receive error for {source_uid}: {e}"
                    );
                    break;
                }
            };

            self.relay_video_datagram(source_uid, datagram).await;
        }
    }

    async fn relay_video_datagram(
        &self,
        source_uid: u64,
        datagram: Bytes,
    ) {
        let viewers = {
            let state = self.state.read().await;

            state.clients
                .values()
                .filter(|client| {
                    client.watching == Some(source_uid)
                })
                .map(|client| client.connection.clone())
                .collect::<Vec<_>>()
        };

        for connection in viewers {
            if let Err(e) = connection.send_datagram(datagram.clone()) {
                eprintln!("failed to relay video datagram: {e}");
            }
        }
    }

}

