use std::time::Instant;

/// One complete video frame.
//#[derive(Clone, Default)]
pub struct RawFrame {
    pub sequence: u64,
    pub timestamp: u64,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

