use std::{
    collections::HashMap,
    sync::{Mutex, RwLock},
};

/// One complete video frame.
#[derive(Clone, Default)]
pub struct SharedFrame {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Shared media state used by the client.
pub struct MediaState {
    /// Latest frame captured from our own screen.
    pub capture: Mutex<SharedFrame>,

    /// Latest frame received from every remote streamer.
    /// Key = streamer's UID.
    pub remote_streams: RwLock<HashMap<u64, SharedFrame>>,
}

impl MediaState {
    pub fn new() -> Self {
        Self {
            capture: Mutex::new(SharedFrame::default()),
            remote_streams: RwLock::new(HashMap::new()),
        }
    }

    /// Replace the latest captured frame.
    pub fn update_capture(&self, frame: SharedFrame) {
        *self.capture.lock().unwrap() = frame;
    }

    /// Get a clone of the latest captured frame.
    pub fn capture(&self) -> SharedFrame {
        self.capture.lock().unwrap().clone()
    }

    /// Update (or create) a remote stream.
    pub fn update_remote(
        &self,
        uid: u64,
        frame: SharedFrame,
    ) {
        self.remote_streams
            .write()
            .unwrap()
            .insert(uid, frame);
    }

    /// Get a copy of a remote stream.
    pub fn remote(
        &self,
        uid: u64,
    ) -> Option<SharedFrame> {
        self.remote_streams
            .read()
            .unwrap()
            .get(&uid)
            .cloned()
    }

    /// Remove a disconnected stream.
    pub fn remove_remote(&self, uid: u64) {
        self.remote_streams
            .write()
            .unwrap()
            .remove(&uid);
    }
}