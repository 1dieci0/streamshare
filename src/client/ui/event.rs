#[derive(Debug, Clone)]
pub enum AppEvent {
    NewFrame(u64),
    StreamStarted(u64),
    StreamStopped(u64),
    UserJoined(u64),
    UserLeft(u64),
}