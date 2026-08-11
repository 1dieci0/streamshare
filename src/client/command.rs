pub enum ClientEvent {
    Connected,
    Authenticated {
        uid: u64,
    },

    UserJoined {
        uid: u64,
        username: String,
    },

    UserLeft {
        uid: u64,
        username: String,
    },

    StreamStarted {
        uid: u64,
        username: String,
    },

    StreamStopped {
        uid: u64,
        username: String,
    },

    VideoFrame{
        frame: crate::protocol::video::VideoPacket,
    },

    Error(String),

    Disconnected,
}

pub enum ClientCommand {
    StartStream,
    StopStream,
    Disconnect,
    WatchStream { uid: u64 },
    LeaveStream { uid: u64 },
}