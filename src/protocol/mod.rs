pub mod udp;
pub mod video;
pub mod tcp;
pub mod tcp_transport;
pub mod udp_transport;

use std::io::Result;

pub trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<()>;

    fn recv(&mut self) -> Result<Vec<u8>>;
}
