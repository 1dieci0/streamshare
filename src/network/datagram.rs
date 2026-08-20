use quinn::Connection;

use crate::protocol::video::VideoPacket;

pub async fn send_frame(
    connection: &Connection,
    packet: VideoPacket
) -> anyhow::Result<()>
{
    let bytes = packet.encode();

    connection
        .send_datagram(bytes.into())?;

    Ok(())
}

pub async fn receive_frames(
    connection: Connection,
) -> anyhow::Result<()>
{
    while let Ok(received_bytes) = connection.read_datagram().await {
        println!("request: {:?}", received_bytes);
        let packet = VideoPacket::decode(&received_bytes);
    }
    Ok(())
}