use std::{
    io::Result,
    net::{UdpSocket, SocketAddr},
};

use super::Transport;


pub struct UdpTransport {

    socket: UdpSocket,
    peer: SocketAddr,

}


impl UdpTransport {

    pub fn new(
        socket: UdpSocket,
        peer: SocketAddr,
    )->Self {

        Self {
            socket,
            peer,
        }
    }

}



impl Transport for UdpTransport {


    fn send(
        &mut self,
        data:&[u8],
    )->Result<()> {

        self.socket
            .send_to(
                data,
                self.peer
            )?;

        Ok(())
    }



    fn recv(
        &mut self,
    )->Result<Vec<u8>> {


        let mut buf =
            vec![0u8;1500];


        let (size, _) =
            self.socket.recv_from(
                &mut buf
            )?;


        buf.truncate(size);


        Ok(buf)
    }

}
