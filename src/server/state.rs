use std::{
    collections::{HashMap, HashSet}, net::SocketAddr,
};
use std::sync::mpsc::{Sender};
use rand::Rng;

use crate::protocol::VideoPacket;

#[derive(Debug, Clone)]
pub enum TcpMessage {
    Authenticated,
    SendUID(u64),
    UserJoined(String),
    UserLeft(String),
    UserStarted(String),
    UserStopped(String),
    Error(String),
}

pub struct Client {
    pub username: String,
    pub session_id: u64,


    pub tcp_sender: Sender<TcpMessage>,
    pub udp_addr: Option<SocketAddr>,
}

pub struct Stream {
    pub latest_packet: Option<VideoPacket>,
}

pub struct ServerState {
    pub users: HashMap<u64, Client>,
    pub streams: HashMap<u64, Stream>,
}

impl ServerState {
    pub fn client_by_addr_mut(
        &mut self,
        addr: SocketAddr,
    ) -> Option<&mut Client> {
        self.users
            .values_mut()
            .find(|client| client.udp_addr == Some(addr))
    }
}

pub fn generate_session_id(state: &ServerState) -> u64 {
    //let mut rng = rand::rng();

    loop {
        let id: u64 = rand::random();

        if !state.users.contains_key(&id) {
            return id;
        }
    }
}