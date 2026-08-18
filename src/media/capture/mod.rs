#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use std::io;
use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    client::command::EncoderCommand,
    media::{
        encoder::EncodedFrame,
        state::Media,
    },
};

pub fn start_capture(
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,
    encoder_rx: Receiver<EncoderCommand>,
) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::start_capture(
            media,
            video_tx,
            encoder_rx,
        )
    }

    #[cfg(target_os = "linux")]
    {
        linux::start_capture(
            media,
            video_tx,
            encoder_rx,
        )
    }
}