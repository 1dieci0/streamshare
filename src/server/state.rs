use std::{
    net::{SocketAddr},
    collections::HashMap,
};
use std::sync::mpsc::{Sender};

#[derive(Debug, Clone)]
pub enum TcpMessage {
    Authenticated,
    UserJoined(String),
    UserLeft(String),
    UserStarted(String),
    UserStopped(String),
    Error(String),
}

pub struct Client {
    pub username: String,

    pub tcp_sender: Sender<TcpMessage>,

    pub udp_addr: Option<SocketAddr>,

    pub streaming: bool,
}

pub struct ServerState {
    pub users: HashMap<String, Client>,
}
