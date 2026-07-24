#[derive(Clone)]
enum Message {
    StartStreaming,
    StopStreaming,
    Audio(Vec<u8>),
}
