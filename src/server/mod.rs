use std::{
    net::{UdpSocket, TcpListener, SocketAddr},
    io::{Result},
    collections::HashMap,
    sync::{Mutex, Arc},
    thread,
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
    state: Arc<Mutex<ServerState>>,
}


impl Server{
    pub fn new(tcp_port: String, udp_port: String) -> Self{
        Self{
            tcp_address: format!("127.0.0.1:{tcp_port}").parse().unwrap(),
            udp_address: format!("127.0.0.1:{udp_port}").parse().unwrap(),
            state: Arc::new(Mutex::new(ServerState {
                users: HashMap::new(),
            })),
        }
    }

    pub fn start(self) -> Result<()> {

        let tcp_listener = TcpListener::bind(self.tcp_address.clone())?;
        let udp_socket = UdpSocket::bind(self.udp_address.clone())?;


        println!("Listening on {}", tcp_listener.local_addr()?);

        // for stream in tcp_listener.incoming() {
        //     let state = Arc::clone(&self.state);
        //
        //     thread::spawn(move || {
        //         match stream {
        //             Ok(stream) => {
        //                 if let Err(e) = Server::handle_client(stream, state) {
        //                     eprintln!("Client error: {e}");
        //                 }
        //             }
        //             Err(e) => eprintln!("Accept error: {e}"),
        //         }
        //     });
        // }

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
