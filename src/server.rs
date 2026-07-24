use std::{
    net::{UdpSocket, TcpListener, TcpStream, SocketAddr},
    io::{prelude::*, ErrorKind, Error, Result},
    collections::HashMap,
    sync::{Mutex, Arc},
    thread,
    sync::mpsc::{channel, Sender},
};

use sha2::{Sha256, Digest};


#[derive(Clone)]
enum Message {
    StartStreaming,
    StopStreaming,
    Audio(Vec<u8>),
}

enum ClientMessage {
    StartStream,
    StopStream,
    VideoFrame(Vec<u8>),
    AudioFrame(Vec<u8>),
}

enum ServerMessage {
    StreamStarted(String),
    StreamStopped(String),
    VideoFrame(Vec<u8>),
    AudioFrame(Vec<u8>),
}


#[derive(Debug, Clone)]
pub enum TcpMessage {
    Authenticated,
    UserJoined(String),
    UserLeft(String),
    UserStarted(String),
    UserStopped(String),
    Error(String),
}

#[derive(Debug)]
enum ClientCommand {
    Authenticate(String),
    StartStreaming,
    StopStreaming,
    SetUdpPort(u16),
    Disconnect,
}


enum UdpPacket {
    VideoFrame {
        frame_number: u32,
        data: Vec<u8>,
    },

    AudioFrame {
        timestamp: u64,
        data: Vec<u8>,
    },
}


struct Client {
    username: String,

    tcp_sender: Sender<TcpMessage>,

    udp_addr: Option<SocketAddr>,

    streaming: bool,
}

struct ServerState {
    users: HashMap<String, Client>,
}

pub struct Server{
    tcp_address: SocketAddr,
    udp_address: SocketAddr,
    state: Arc<Mutex<ServerState>>,
}

const SERVER_KEY: &str = "super_secret_key";

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
            Self::udp_loop(udp_socket, udp_state);
        });

        for stream in tcp_listener.incoming() {
            let state = Arc::clone(&self.state);

            thread::spawn(move || {
                Self::handle_tcp(stream.unwrap(), state);
            });
        }

        Ok(())
    }

    fn udp_loop(
        udp_socket: UdpSocket,
        udp_state: Arc<Mutex<ServerState>>
    )-> Result<()> {

        Ok(())
    }





    fn handle_tcp(
        mut stream: TcpStream,
        state: Arc<Mutex<ServerState>>,
    ) -> Result<()> {

        /*
            AUTHENTICATION PHASE
        */

        let challenge = generate_challenge();

        // Send challenge
        stream.write_all(&challenge)?;


        // Receive username length
        let mut username_len = [0u8; 2];
        stream.read_exact(&mut username_len)?;

        let username_len =
            u16::from_be_bytes(username_len) as usize;


        // Receive username
        let mut username_buf = vec![0u8; username_len];
        stream.read_exact(&mut username_buf)?;

        let username =
            String::from_utf8(username_buf)
                .unwrap();


        // Receive hash response
        let mut response = [0u8; 32];
        stream.read_exact(&mut response)?;


        // Verify
        let expected =
            create_response(
                SERVER_KEY,
                &challenge,
            );


        if response != expected[..] {
            stream.write_all(&[0x00])?; // denied
            return Ok(());
        }


        stream.write_all(&[0x01])?; // accepted


        /*
            CREATE MPSC
        */

        let (tx, rx) =
            channel::<TcpMessage>();


        /*
            WRITER THREAD
        */

        let mut write_stream =
            stream.try_clone()?;


        thread::spawn(move || {

            let mut message_to_send;
            let mut message_bytes;

            while let Ok(message) = rx.recv() {

                match message {

                    TcpMessage::Authenticated => {
                        let _ =
                            write_stream.write_all(&[0x10]);
                    }


                    TcpMessage::UserJoined(name) => {
                        message_to_send = format!("{name} joined");
                        message_bytes = message_to_send.as_bytes();

                        println!("{message_to_send}");

                        if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }

                        if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }
                    }


                    TcpMessage::UserLeft(name) => {
                        message_to_send = format!("{name} left");
                        message_bytes = message_to_send.as_bytes();

                        println!("{message_to_send}");

                        if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }

                        if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }
                    }


                    TcpMessage::UserStarted(name) => {
                        message_to_send = format!("{name} started streaming");
                        message_bytes = message_to_send.as_bytes();
                        
                        println!("{message_to_send}");

                        if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }

                        if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }
                    }


                    TcpMessage::UserStopped(name) => {
                        message_to_send = format!("{name} stopped streaming");
                        message_bytes = message_to_send.as_bytes();

                        println!("{message_to_send}");

                        if let Err(e) = write_stream.write_all(&(message_bytes.len() as u32).to_be_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }

                        if let Err(e) = write_stream.write_all(message_to_send.as_bytes()) {
                            println!("Connection closed: {e}");
                            break;
                        }
                    }


                    TcpMessage::Error(err) => {
                        println!("error: {}", err);
                    }
                }
            }
        });



        /*
            REGISTER CLIENT
        */

        {
            let mut state =
                state.lock().unwrap();


            state.users.insert(
                username.clone(),
                Client {
                    username: username.clone(),
                    tcp_sender: tx.clone(),
                    udp_addr: None,
                    streaming: false,
                }
            );


            // notify existing users
            for (name, client)
                in &state.users
            {
                if name != &username {
                    let _ =
                        client.tcp_sender.send(
                            TcpMessage::UserJoined(
                                username.clone()
                            )
                        );
                }
            }
        }



        /*
            NORMAL COMMAND LOOP
        */

        loop {

            let mut command = [0u8; 1];


            if stream.read_exact(&mut command).is_err() {
                break;
            }


            match command[0] {


                // START STREAM
                0x01 => {


                    let mut state =
                        state.lock().unwrap();


                    if let Some(client)
                        = state.users.get_mut(&username)
                    {
                        if client.streaming == true{
                            continue;
                        }
                        client.streaming = true;
                    }


                    for client in state.users.values()
                    {
                        let _ =
                            client.tcp_sender.send(
                                TcpMessage::UserStarted(
                                    username.clone()
                                )
                            );
                    }
                }


                // STOP STREAM
                0x02 => {

                    let mut state =
                        state.lock().unwrap();


                    if let Some(client)
                        = state.users.get_mut(&username)
                    {
                        if client.streaming == false{
                            continue;
                        }
                        client.streaming = false;
                    }


                    for client in state.users.values()
                    {
                        let _ =
                            client.tcp_sender.send(
                                TcpMessage::UserStopped(
                                    username.clone()
                                )
                            );
                    }
                }


                // DISCONNECT
                0x03 => {

                    let mut state =
                        state.lock().unwrap();


                    state.users.remove(&username);


                    for client in state.users.values()
                    {
                        let _ =
                            client.tcp_sender.send(
                                TcpMessage::UserLeft(
                                    username.clone()
                                )
                            );
                    }

                    break;
                }


                _ => {}
            }
        }



        /*
            CLEANUP
        */

        let mut state =
            state.lock().unwrap();


        state.users.remove(&username);


        for client in state.users.values()
        {
            let _ =
                client.tcp_sender.send(
                    TcpMessage::UserLeft(
                        username.clone()
                    )
                );
        }


        Ok(())
    }

                
}

fn authenticate(key: &str) -> bool {
    key == SERVER_KEY
}


fn generate_challenge() -> [u8; 32] {
    let mut challenge = [0u8; 32];

    rand::fill(&mut challenge);

    challenge
}


fn create_response(key: &str, challenge: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();

    hasher.update(key.as_bytes());
    hasher.update(challenge);

    hasher.finalize().to_vec()
}
