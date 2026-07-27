pub struct VideoPacket {
    pub uid: u64,
    pub sequence: u64,
    pub data: Vec<u8>,
}

impl VideoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + self.data.len());

        out.extend_from_slice(&self.uid.to_be_bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        out.extend_from_slice(&self.data);

        out
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 12 {
            return None;
        }

        let uid = u64::from_be_bytes(buf[0..8].try_into().ok()?);
        let sequence = u64::from_be_bytes(buf[8..12].try_into().ok()?);
        let data = buf[12..].to_vec();

        Some(Self {
            uid,
            sequence,
            data,
        })
    }
}