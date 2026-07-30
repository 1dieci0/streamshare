use std::{
    collections::HashMap, sync::{Arc, RwLock},
};

use crate::media::frame::SharedFrame;


/// Shared media state used by the client.
pub struct MediaState {
    pub outgoing: RwLock<Option<Arc<SharedFrame>>>,
    pub incoming: RwLock<HashMap<u64, Arc<SharedFrame>>>,
}

impl MediaState {
    pub fn new() -> Self {
        Self {
            outgoing: RwLock::new(None),
            incoming: RwLock::new(HashMap::new()),
        }
    }

    /// Replace the latest captured frame.
    pub fn update_capture(&self, frame: SharedFrame) {
        *self.outgoing.write().unwrap() = Some(Arc::new(frame));
    }

    /// Get a clone of the latest captured frame.
    pub fn capture(&self) -> Option<Arc<SharedFrame>> {
        self.outgoing.read().unwrap().clone()
    }

    /// Update (or create) a remote stream.
    pub fn update_remote(
        &self,
        uid: u64,
        frame: SharedFrame,
    ) {
        self.incoming
            .write()
            .unwrap()
            .insert(uid, Arc::new(frame));
    }

    // /// Get a copy of a remote stream.
    // pub fn remote(
    //     &self,
    //     uid: u64,
    // ) -> Option<SharedFrame> {
    //     self.remote_streams
    //         .read()
    //         .unwrap()
    //         .get(&uid)
    //         .cloned()
    // }

    /// Remove a disconnected stream.
    pub fn remove_remote(&self, uid: u64) {
        self.incoming
            .write()
            .unwrap()
            .remove(&uid);
    }

    pub fn stream_ids(&self) -> Vec<u64> {
        self.incoming
            .read()
            .unwrap()
            .keys()
            .copied()
            .collect()
    }

    pub fn incoming(&self, uid: u64) -> Option<Arc<SharedFrame>> {
        self.incoming
            .read()
            .unwrap()
            .get(&uid)
            .cloned()
    }

    pub fn has_stream(&self, uid:u64)->bool {
        self.incoming
            .read()
            .unwrap()
            .contains_key(&uid)
    }

    pub fn clear_remote(&self){
        self.incoming
            .write()
            .unwrap()
            .clear();
    }
}