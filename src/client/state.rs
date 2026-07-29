use std::sync::atomic::{AtomicBool, AtomicU64};

pub struct ClientState{
    pub uid: AtomicU64,
    pub streaming: AtomicBool,
    pub sequence: AtomicU64,
}

impl ClientState{
    pub fn new() -> ClientState{
        ClientState {
            uid: AtomicU64::new(0),
            streaming: AtomicBool::new(false),
            sequence: AtomicU64::new(0),
        }
    }
}