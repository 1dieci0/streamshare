#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VideoCodec {
    H264 = 1,
}

impl VideoCodec {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::H264),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

pub struct VideoPacket {
    pub uid: u64,

    // identifies the complete encoded frame
    pub frame_id: u64,

    // fragmentation
    pub packet_index: u16,
    pub packet_total: u16,

    // decoder metadata
    pub codec: u8,
    pub width: u32,
    pub height: u32,

    pub data: Vec<u8>,
}

impl VideoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(29 + self.data.len());

        out.extend_from_slice(&self.uid.to_be_bytes());
        out.extend_from_slice(&self.frame_id.to_be_bytes());

        out.extend_from_slice(&self.packet_index.to_be_bytes());
        out.extend_from_slice(&self.packet_total.to_be_bytes());

        out.push(self.codec);
        out.extend_from_slice(&self.width.to_be_bytes());
        out.extend_from_slice(&self.height.to_be_bytes());

        out.extend_from_slice(&self.data);

        out
    }


    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 29 {
            return None;
        }

        let uid =
            u64::from_be_bytes(buf[0..8].try_into().ok()?);

        let frame_id =
            u64::from_be_bytes(buf[8..16].try_into().ok()?);

        let packet_index =
            u16::from_be_bytes(buf[16..18].try_into().ok()?);

        let packet_total =
            u16::from_be_bytes(buf[18..20].try_into().ok()?);

        let codec = buf[20];

        let width =
            u32::from_be_bytes(buf[21..25].try_into().ok()?);

        let height =
            u32::from_be_bytes(buf[25..29].try_into().ok()?);

        let data = buf[29..].to_vec();


        Some(Self {
            uid,
            frame_id,
            packet_index,
            packet_total,
            codec,
            width,
            height,
            data,
        })
    }
}