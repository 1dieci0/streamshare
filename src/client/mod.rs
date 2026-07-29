use std::io::Read;
use std::net::TcpStream;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::sync::atomic::{AtomicBool};

mod auth;
use auth::authenticate;
mod capture;
mod playback;
mod tcp;
use tcp::handle_tcp;
mod udp;
use udp::udp_loop;
pub mod state;

mod ui;

use crate::client::auth::get_uid;
use crate::media;
use crate::protocol::udp::UdpPacket;

pub struct Client{
    pub username: String,
    pub tcp_addr: SocketAddr,
    pub udp_addr: SocketAddr,

    pub client_state: Arc<state::ClientState>,
    pub app_state: Arc<RwLock<ui::state::AppState>>,
    pub media_state: Arc<media::state::MediaState>,
}

// pub struct ClientState{
//     pub uid: u64,
//     pub streaming: AtomicBool,
//     pub sequence: u64,
// }

impl Client{
    pub fn new(username: String, server_address: String, tcp_port: String, udp_port: String) -> Client{
        Client{
            username,
            tcp_addr: format!("{server_address}:{tcp_port}").parse().unwrap(),
            udp_addr: format!("{server_address}:{udp_port}").parse().unwrap(),
            client_state: Arc::new(state::ClientState::new()),
            app_state: Arc::new(RwLock::new(ui::state::AppState::new())),
            media_state: Arc::new((media::state::MediaState::new())),
        }
    }

    pub fn start(mut self) -> std::io::Result<()>{

        let mut tcp = TcpStream::connect(self.tcp_addr)?;

        let udp = UdpSocket::bind("0.0.0.0:0")?;

        authenticate(&mut tcp, &self.username)?;

        get_uid(
            &self.client_state.as_ref(),
            &mut tcp,
            &udp,
            self.udp_addr
        )?;

        media::capture::start_capture(
            Arc::clone(&self.media_state),
            Arc::clone(&self.client_state),
        );


        udp::start_sender(
            udp.try_clone()?,
            self.udp_addr,
            Arc::clone(&self.client_state),
            Arc::clone(&self.app_state),
            Arc::clone(&self.media_state),
        );

        udp::start_receiver(
            udp,
            self.udp_addr,
            Arc::clone(&self.client_state),
            Arc::clone(&self.app_state),
            Arc::clone(&self.media_state),
        );

        tcp::start_sender(
            tcp.try_clone()?,
            Arc::clone(&self.client_state),
            Arc::clone(&self.app_state)
        );

        tcp::start_receiver(
            tcp,
            Arc::clone(&self.app_state),
        );

        ui::start_ui(
            self.client_state,
            self.app_state,
            self.media_state,
        );



        Ok(())
    }

}


