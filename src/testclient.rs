// use std::net::UdpSocket;
// use std::net::TcpStream;
// use std::io::prelude::*;
// use std::io::Result;
// use std::io::stdin;
// use std::net::SocketAddr;
// use std::thread;
//
// use sha2::{Digest, Sha256};
//
// pub struct Client{
//     username: String,
//     tcp_addr: SocketAddr,
//     udp_addr: SocketAddr,
// }
//
// const SERVER_KEY: &str = "super_secret_key";
//
// fn create_response(key: &str, challenge: &[u8]) -> Vec<u8> {
//     let mut hasher = Sha256::new();
//
//     hasher.update(key.as_bytes());
//     hasher.update(challenge);
//
//     hasher.finalize().to_vec()
// }
//
// fn authenticate(
//     stream: &mut TcpStream,
//     username: &str,
// ) -> Result<bool> {
//
//     // Receive challenge
//     let mut challenge = [0u8; 32];
//
//     stream.read_exact(&mut challenge)?;
//
//
//     // Create response
//     let response = create_response(
//         SERVER_KEY,
//         &challenge,
//     );
//
//
//     // Send username length
//     let username_bytes = username.as_bytes();
//
//     let len = username_bytes.len() as u16;
//
//     stream.write_all(
//         &len.to_be_bytes()
//     )?;
//
//
//     // Send username
//     stream.write_all(username_bytes)?;
//
//
//     // Send hash response
//     stream.write_all(&response)?;
//
//
//     // Receive server answer
//     let mut result = [0u8; 1];
//
//     stream.read_exact(&mut result)?;
//
//
//     Ok(result[0] == 0x01)
// }
//
//
//
// fn send_command(
//     stream: &mut TcpStream,
//     command: u8,
// ) -> Result<()> {
//
//     stream.write_all(&[command])?;
//
//     Ok(())
// }
//
//
//
// impl Client{
//     pub fn new(username: String, server_address: String, tcp_port: String, udp_port: String) -> Client{
//         Client{
//             username,
//             tcp_addr: format!("{server_address}:{tcp_port}").parse().unwrap(),
//             udp_addr: format!("{server_address}:{udp_port}").parse().unwrap(),
//         }
//     }
//
//     pub fn start(self) -> std::io::Result<()>{
//
//         let mut stream =
//             TcpStream::connect(self.tcp_addr)?;
//
//
//         println!("Connected");
//
//
//         if !authenticate(
//             &mut stream,
//             &self.username,
//         )? {
//             println!("Authentication failed");
//             return Ok(());
//         }
//
//
//         println!("Authenticated!");
//
//
//         let mut receive_stream =
//             stream.try_clone()?;
//
//         thread::spawn(move || {
//             let mut len_buf = [0u8; 4];
//
//             loop {
//
//                 if let Err(e) = receive_stream.read_exact(&mut len_buf) {
//                     println!("Connection closed: {e}");
//                     break;
//                 }
//
//                 let len = u32::from_be_bytes(len_buf) as usize;
//                 let mut buffer = vec![0u8; len];
//
//                 if let Err(e) = receive_stream.read_exact(&mut buffer) {
//                     println!("Connection closed: {e}");
//                     break;
//                 }
//
//                 println!("{}", String::from_utf8_lossy(&buffer));
//             }
//         });
//
//
//         loop {
//
//             println!();
//             println!("1 - start streaming");
//             println!("2 - stop streaming");
//             println!("3 - disconnect");
//
//
//             let mut input = String::new();
//
//             stdin()
//                 .read_line(&mut input)?;
//
//
//             match input.trim() {
//
//                 "1" => {
//                     // START_STREAM
//                     send_command(
//                         &mut stream,
//                         0x01,
//                     )?;
//
//                     println!("Streaming started");
//                 }
//
//
//                 "2" => {
//                     // STOP_STREAM
//                     send_command(
//                         &mut stream,
//                         0x02,
//                     )?;
//
//                     println!("Streaming stopped");
//                 }
//
//
//                 "3" => {
//                     // DISCONNECT
//                     send_command(
//                         &mut stream,
//                         0x03,
//                     )?;
//
//                     break;
//                 }
//
//
//                 _ => {
//                     println!("Invalid command");
//                 }
//             }
//         }
//
//
//         Ok(())
//     }
//
// }
//
//
