use std::{
    io::{Read, Result, Write},
    net::TcpStream,
};

use super::Transport;


pub struct TcpTransport {
    stream: TcpStream,
}


impl TcpTransport {

    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
        }
    }

}


impl Transport for TcpTransport {

    fn send(
        &mut self,
        data: &[u8],
    ) -> Result<()> {

        let len = data.len() as u32;


        self.stream.write_all(
            &len.to_be_bytes()
        )?;


        self.stream.write_all(data)?;

        Ok(())
    }


    fn recv(
        &mut self,
    ) -> Result<Vec<u8>> {


        let mut len_buf = [0u8;4];

        self.stream.read_exact(
            &mut len_buf
        )?;


        let len =
            u32::from_be_bytes(len_buf)
            as usize;


        let mut buffer =
            vec![0u8;len];


        self.stream.read_exact(
            &mut buffer
        )?;


        Ok(buffer)
    }
}
