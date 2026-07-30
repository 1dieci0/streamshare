/// One complete video frame.
#[derive(Clone, Default)]
pub struct SharedFrame {
    pub sequence: u64,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

