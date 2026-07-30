use std::collections::{HashMap, VecDeque};

pub struct AppState{
    pub selected_stream: Option<u64>,
    pub available_streams: HashMap<u64, StreamInfo>,
    pub notifications: VecDeque<String>,
}


pub struct StreamInfo {
    pub uid: u64,
    pub username: String,
    pub resolution: (u32, u32),
    pub fps: u32,
}

impl AppState{
    pub fn new() -> AppState{
        AppState { 
            selected_stream: None,
            available_streams: HashMap::new(),
            notifications: VecDeque::new(),
        }
    }
}