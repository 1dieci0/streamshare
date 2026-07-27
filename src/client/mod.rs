use std::io::Read;
use std::net::TcpStream;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};

mod auth;
use auth::authenticate;
mod capture;
mod playback;
mod tcp;
use tcp::handle_tcp;
mod udp;
use udp::udp_loop;

pub struct Client{
    pub username: String,
    pub tcp_addr: SocketAddr,
    pub udp_addr: SocketAddr,

    pub state: Arc<RwLock<ClientState>>,
}

pub struct ClientState{
    pub uid: u64,
    pub streaming: AtomicBool,
    pub sequence: u64,
}

impl Client{
    pub fn new(username: String, server_address: String, tcp_port: String, udp_port: String) -> Client{
        Client{
            username,
            tcp_addr: format!("{server_address}:{tcp_port}").parse().unwrap(),
            udp_addr: format!("{server_address}:{udp_port}").parse().unwrap(),
            state: Arc::new(RwLock::new(ClientState {
                            uid: 0,
                            streaming: AtomicBool::new(false),
                            sequence: 0,
            })),
        }
    }

    pub fn start(mut self) -> std::io::Result<()>{

        let mut stream =
            TcpStream::connect(self.tcp_addr)?;

        let udp_socket = UdpSocket::bind("127.0.0.1:0")?;


        println!("Connected");


        if !authenticate(
            &mut stream,
            &self.username,
        )? {
            println!("Authentication failed");
            return Ok(());
        }


        println!("Authenticated!");

        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf)?;

        {
            self.state.write().unwrap().uid = u64::from_be_bytes(buf);
        }

        let udp_state = Arc::clone(&self.state);

        thread::spawn(move || {
            let _ = udp_loop(udp_state, self.udp_addr, udp_socket);
        });

         let tcp_state = Arc::clone(&self.state);

        if let Err(e) = handle_tcp(stream, tcp_state) {
            println!("Connection closed: {e}");
        }


        Ok(())
    }

}


