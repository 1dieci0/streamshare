use crate::protocol::video::VideoPacket;

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum PacketType {
    Register = 1,
    Video = 2,
    Heartbeat = 3,
}


impl PacketType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(PacketType::Register),
            2 => Some(PacketType::Video),
            3 => Some(PacketType::Heartbeat),
            _ => None,
        }
    }
}

pub enum UdpPacket{
    Register{
        uid: u64,
    },

    Video(VideoPacket),

    Heartbeat,
}

impl UdpPacket {
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        let packet_type = PacketType::from_u8(buf[0])?;

        match packet_type {
            PacketType::Register => {
                if buf.len() < 9 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                Some(UdpPacket::Register { uid })
            }

            PacketType::Video => {
                Some(UdpPacket::Video(
                    VideoPacket::decode(&buf[1..])?
                ))
            }

            PacketType::Heartbeat => {
                Some(UdpPacket::Heartbeat)
            }
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            UdpPacket::Register { uid } => {
                let mut out = Vec::with_capacity(9);

                out.push(PacketType::Register as u8);
                out.extend_from_slice(&uid.to_be_bytes());

                out
            }

            UdpPacket::Video(video) => {
                let video_data = video.encode();

                let mut out = Vec::with_capacity(1 + video_data.len());

                out.push(PacketType::Video as u8);
                out.extend_from_slice(&video_data);

                out
            }

            UdpPacket::Heartbeat => {
                vec![PacketType::Heartbeat as u8]
            }
        }
    }
}