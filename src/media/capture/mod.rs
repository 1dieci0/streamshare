use std::{
    io,
    sync::Arc,
    thread,
};


mod ffmpeg_encoder;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{client::command::EncoderCommand, media::{capture::ffmpeg_encoder::FFmpegEncoder, encoder::EncodedFrame, state::Media}};

pub fn start_capture(
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,
    _encoder_rx: Receiver<EncoderCommand>,
) -> io::Result<()> {
    tokio::spawn(async move {
        
        while !media.is_streaming() {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        println!("Starting FFmpeg capture...");

        let encoder = match FFmpegEncoder::start(
            1920,
            1080,
            60,
            5000,
            video_tx,
        )
        .await
        {
            Ok(encoder) => encoder,

            Err(e) => {
                eprintln!("Failed to start FFmpeg: {e}");
                return;
            }
        };

        println!("FFmpeg capture started");

        // Keep the encoder alive while streaming.
        while media.is_streaming() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        drop(encoder);

        println!("FFmpeg capture stopped");
    });

    Ok(())
}