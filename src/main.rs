use std::result;
use std::{
    net::TcpListener, 
    net::TcpStream, 
    net::UdpSocket,
    collections::HashMap, 
    path::PathBuf,
    thread,
    env,
    time::Duration,
};
use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};

mod server;
mod client;


#[derive(Parser, Debug)]
#[command(name = "streamshare")]
#[command(about = "Video/audio streaming application")]
struct Args {

    #[command(subcommand)]
    command: Commands,
}


#[derive(Subcommand, Debug)]
enum Commands {

    /// Start the client
    Client {
        /// username
        #[arg(long)]
        username: String,
        /// Server address
        #[arg(long)]
        address: String,

        /// TCP port
        #[arg(long)]
        tcp_port: u16,

        /// UDP port
        #[arg(long)]
        udp_port: u16,
    },


    /// Start the server
    Server {
        /// TCP port
        #[arg(long)]
        tcp_port: u16,

        /// UDP port
        #[arg(long)]
        udp_port: u16,
    },
}

fn main() -> std::io::Result<()> {

    let args = Args::parse();


    match args.command {

        Commands::Client {
            username,
            address,
            tcp_port,
            udp_port,
        } => {

            println!("Starting client");

            println!("Server: {}", address);
            println!("TCP: {}", tcp_port);
            println!("UDP: {}", udp_port);


            let s = client::Client::new(username.to_string(), address.to_string(), tcp_port.to_string(), udp_port.to_string());
            s.start()?;
        }


        Commands::Server {
            tcp_port,
            udp_port,
        } => {

            println!("Starting server");

            println!("TCP: {}", tcp_port);
            println!("UDP: {}", udp_port);

            let s = server::Server::new(tcp_port.to_string(), udp_port.to_string());
            s.start()?;
        }
    }

    Ok(())
}

#[test]
fn client_server() {
    let server = thread::spawn(||  {
        let socket = UdpSocket::bind("127.0.0.1:40000").unwrap();

        let mut buf = [0; 1024];
        let (len, addr) = socket.recv_from(&mut buf).unwrap();

        assert_eq!(&buf[..len], b"ping");

        socket.send_to(b"pong", addr).unwrap();
    });

    thread::sleep(Duration::from_millis(50));

    let client = UdpSocket::bind("127.0.0.1:0").unwrap();

    client.send_to(b"ping", "127.0.0.1:40000").unwrap();

    let mut buf = [0; 1024];
    let (len, _) = client.recv_from(&mut buf).unwrap();

    assert_eq!(&buf[..len], b"pong");

    server.join().unwrap();
}
