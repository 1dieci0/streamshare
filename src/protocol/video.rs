#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
}

impl VideoCodec {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::H264 => 1,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::H264),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VideoPacket {
    /// UID of the user who owns the stream.
    pub uid: u64,

    /// Identifies the complete encoded frame.
    pub frame_id: u64,

    /// Presentation timestamp / capture timestamp.
    pub timestamp: u64,

    /// Fragment index within this frame.
    pub packet_index: u16,

    /// Total number of fragments belonging to this frame.
    pub packet_total: u16,

    /// Whether this encoded frame is a keyframe.
    pub keyframe: bool,

    /// Codec used for the encoded payload.
    pub codec: VideoCodec,

    /// Original video dimensions.
    pub width: u32,
    pub height: u32,

    /// Fragment of the encoded H.264 bitstream.
    pub data: Vec<u8>,
}


impl VideoPacket {
    pub const HEADER_SIZE: usize = 38;

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            Self::HEADER_SIZE + self.data.len()
        );

        out.extend_from_slice(&self.uid.to_be_bytes());
        out.extend_from_slice(&self.frame_id.to_be_bytes());
        out.extend_from_slice(&self.timestamp.to_be_bytes());

        out.extend_from_slice(&self.packet_index.to_be_bytes());
        out.extend_from_slice(&self.packet_total.to_be_bytes());

        out.push(self.keyframe as u8);
        out.push(self.codec.as_u8());

        out.extend_from_slice(&self.width.to_be_bytes());
        out.extend_from_slice(&self.height.to_be_bytes());

        out.extend_from_slice(&self.data);

        out
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < Self::HEADER_SIZE {
            return None;
        }

        let uid =
            u64::from_be_bytes(buf[0..8].try_into().ok()?);

        let frame_id =
            u64::from_be_bytes(buf[8..16].try_into().ok()?);

        let timestamp =
            u64::from_be_bytes(buf[16..24].try_into().ok()?);

        let packet_index =
            u16::from_be_bytes(buf[24..26].try_into().ok()?);

        let packet_total =
            u16::from_be_bytes(buf[26..28].try_into().ok()?);

        let keyframe = match buf[28] {
            0 => false,
            1 => true,
            _ => return None,
        };

        let codec =
            VideoCodec::from_u8(buf[29])?;

        let width =
            u32::from_be_bytes(buf[30..34].try_into().ok()?);

        let height =
            u32::from_be_bytes(buf[34..38].try_into().ok()?);

        if packet_total == 0 {
            return None;
        }

        if packet_index >= packet_total {
            return None;
        }

        let data = buf[Self::HEADER_SIZE..].to_vec();

        Some(Self {
            uid,
            frame_id,
            timestamp,
            packet_index,
            packet_total,
            keyframe,
            codec,
            width,
            height,
            data,
        })
    }
}