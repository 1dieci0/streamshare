use std::net::TcpStream;
use std::net::SocketAddr;
use std::thread;

mod auth;
use auth::authenticate;
mod capture;
mod playback;
mod tcp;
use tcp::handle_tcp;
mod udp;
use udp::udp_loop;

pub struct Client{
    username: String,
    tcp_addr: SocketAddr,
    udp_addr: SocketAddr,
}

impl Client{
    pub fn new(username: String, server_address: String, tcp_port: String, udp_port: String) -> Client{
        Client{
            username,
            tcp_addr: format!("{server_address}:{tcp_port}").parse().unwrap(),
            udp_addr: format!("{server_address}:{udp_port}").parse().unwrap(),
        }
    }

    pub fn start(self) -> std::io::Result<()>{

        let mut stream =
            TcpStream::connect(self.tcp_addr)?;


        println!("Connected");


        if !authenticate(
            &mut stream,
            &self.username,
        )? {
            println!("Authentication failed");
            return Ok(());
        }


        println!("Authenticated!");


        // thread::spawn(move || {
        //     let _ = udp_loop(self.udp_addr);
        // });

        if let Err(e) = handle_tcp(stream) {
            println!("Connection closed: {e}");
        }


        Ok(())
    }

}


