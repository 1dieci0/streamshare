use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct Media {
    streaming: AtomicBool,
}

impl Media {
    pub fn new() -> Self {
        Self {
            streaming: AtomicBool::new(false),
        }
    }

    pub fn start_streaming(&self) {
        self.streaming.store(true, Ordering::Release);
    }

    pub fn stop_streaming(&self) {
        self.streaming.store(false, Ordering::Release);
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming.load(Ordering::Acquire)
    }
}