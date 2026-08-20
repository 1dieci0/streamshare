use quinn::Connection;

async fn open_bidirectional_stream(connection: Connection) -> anyhow::Result<()> {
    let (mut send, mut recv) = connection.open_bi().await?;
    send.write_all(b"test").await?;
    send.finish()?;
    let received = recv.read_to_end(10).await?;
    Ok(())
}
async fn receive_bidirectional_stream(connection: Connection) -> anyhow::Result<()> {
    while let Ok((mut send, mut recv)) = connection.accept_bi().await {
        // Because it is a bidirectional stream, we can both send and receive.
        println!("request: {:?}", recv.read_to_end(50).await?);
        send.write_all(b"response").await?;
        send.finish()?;
    }
    Ok(())
}


use anyhow::{bail, Result};
use quinn::{RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::command::Packet;

pub async fn send_packet<P: Packet>(
    send: &mut SendStream,
    packet: &P,
) -> Result<()> {

    let bytes = packet.encode();

    send.write_u32(bytes.len() as u32).await?;
    send.write_all(&bytes).await?;
    send.flush().await?;

    Ok(())
}

pub async fn receive_packet<P: Packet>(
    recv: &mut RecvStream,
) -> Result<P> {

    let len = recv.read_u32().await? as usize;

    if len > 64 * 1024 {
        bail!("command packet too large");
    }

    let mut buffer = vec![0u8; len];

    recv.read_exact(&mut buffer).await?;

    P::decode(&buffer).ok_or_else(|| anyhow::anyhow!("invalid packet"))
}