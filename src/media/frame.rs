/// One complete video frame.
#[derive(Clone, Default)]
pub struct VideoFrame {
    pub uid: u64,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

