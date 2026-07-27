use std::{
    collections::HashMap, io::Result, net::{SocketAddr, TcpListener, UdpSocket}, sync::{Arc, RwLock}, thread,
};


pub mod state;
pub mod tcp;
use tcp::handle_tcp;
pub mod udp;
use udp::udp_loop;
pub mod auth;

use state::ServerState;

pub struct Server{
    tcp_address: SocketAddr,
    udp_address: SocketAddr,
    state: Arc<RwLock<ServerState>>,
}


impl Server{
    pub fn new(tcp_port: String, udp_port: String) -> Self{
        Self{
            tcp_address: format!("127.0.0.1:{tcp_port}").parse().unwrap(),
            udp_address: format!("127.0.0.1:{udp_port}").parse().unwrap(),
            state: Arc::new(RwLock::new(ServerState {
                users: HashMap::new(),
                streams: HashMap::new(),
            })),
        }
    }

    pub fn start(self) -> Result<()> {

        let tcp_listener = TcpListener::bind(self.tcp_address.clone())?;
        let udp_socket = UdpSocket::bind(self.udp_address.clone())?;


        println!("Listening on {}", tcp_listener.local_addr()?);


        let udp_state = Arc::clone(&self.state);

        thread::spawn(move || {
            if let Err(e) = udp_loop(udp_socket, udp_state) {
                println!("Connection closed: {e}");
            }
        });

        for stream in tcp_listener.incoming() {
            let state = Arc::clone(&self.state);

            thread::spawn(move || {
                if let Err(e) = handle_tcp(stream.unwrap(), state) {
                    println!("Connection closed: {e}");
                }
            });
        }

        Ok(())
    }
}
