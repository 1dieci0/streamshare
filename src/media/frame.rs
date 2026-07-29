pub struct SharedFrame {
    pub data: Option<Vec<u8>>,
}


pub struct RemoteStream {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub frame: Vec<u8>,
}

