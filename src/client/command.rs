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

    InitialState { 
        users: Vec<crate::protocol::info::UserInfo>,
        streams: Vec<crate::protocol::info::StreamInfo>,
    },

    Error(String),

    Disconnected,

    WatchStream{
        uid: u64,
        username: String,
        stream_uid: u64,
        stream_username: String,
    }
}

pub enum ClientCommand {
    StartStream,
    StopStream,
    Disconnect,
    WatchStream { uid: u64 },
    LeaveStream { uid: u64 },
}

pub enum EncoderCommand {
    ForceKeyframe,
}