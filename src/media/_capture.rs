use std::{
    io,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use scap::{
    capturer::{Capturer, Options, Resolution}, frame::{Frame, FrameType},
};

use tokio::sync::mpsc::{Sender, Receiver};

use crate::{client::command::EncoderCommand, media::{
    encoder::{EncodedFrame, VideoEncoder},
    frame::RawFrame,
    state::Media,
}};

pub struct Screen {
    capturer: Capturer,
}

impl Screen {
    pub fn new() -> io::Result<Self> {
        if !scap::is_supported() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "screen capture is not supported",
            ));
        }

        if !scap::has_permission() {
            if !scap::request_permission() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "screen capture permission denied",
                ));
            }
        }

        let options = Options {
            fps: 144,
            target: None,
            show_cursor: true,
            show_highlight: false,
            excluded_targets: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::Captured,
            ..Default::default()
        };

        let mut capturer = Capturer::new(options);

        capturer.start_capture();

        Ok(Self { capturer })
    }

    pub fn current_frame(&self) -> io::Result<RawFrame> {
        let frame = self
            .capturer
            .get_next_frame()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("scap capture error: {e:?}"),
                )
            })?;

        match frame {
            Frame::BGRA(frame) => {
                Ok(RawFrame {
                    sequence: 0,
                    timestamp: 0,
                    width: frame.width as usize,
                    height: frame.height as usize,
                    data: frame.data,
                })
            }

            other => {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected capture format: {other:?}"),
                ))
            }
        }
    }
}

pub fn start_capture(
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,
    encoder_rx: Receiver<EncoderCommand>,
) {
    thread::spawn(move || {
        let Ok(mut screen) = Screen::new() else {
            eprintln!("Failed to initialize screen capture");
            return;
        };

        let mut encoder = match VideoEncoder::new(encoder_rx) {
            Ok(encoder) => encoder,

            Err(e) => {
                eprintln!("Failed to initialize H264 encoder: {e}");
                return;
            }
        };

        let mut sequence = 0u64;
        let mut stream_start = Instant::now();

        let mut captured = 0u64;
        let mut last_report  = Instant::now();

        loop {
            // ---------------------------------------------------------
            // Not streaming
            // ---------------------------------------------------------

            if !media.is_streaming() {
                thread::sleep(Duration::from_millis(50));

                sequence = 0;
                stream_start = Instant::now();

                continue;
            }

            // ---------------------------------------------------------
            // Capture
            //
            // scap is configured for 60 FPS, so get_next_frame()
            // waits for the next capture frame.
            // ---------------------------------------------------------

            let mut frame = match screen.current_frame() {
                Ok(frame) => frame,

                Err(e) => {
                    eprintln!("capture error: {e}");
                    continue;
                }
            };

            captured += 1;
            if last_report.elapsed() >= Duration::from_secs(1){
                println!("capture fps: {captured}");
                captured = 0;
                last_report = Instant::now();
            }



            frame.sequence = sequence;
            frame.timestamp =
                stream_start.elapsed().as_micros() as u64;


            continue;

            // println!("captured frame {}", sequence);

            // ---------------------------------------------------------
            // H264 encode
            // ---------------------------------------------------------

            let encoded = match encoder.encode_frame(&frame) {
                Ok(encoded) => encoded,

                Err(e) => {
                    eprintln!("H264 encoding error: {e}");
                    continue;
                }
            };

            if encoded.data.is_empty() {
                continue;
            }

            // println!(
            //     "ENCODED frame={} bytes={} keyframe={}",
            //     encoded.sequence,
            //     encoded.data.len(),
            //     encoded.keyframe
            // );

            sequence += 1;

            // ---------------------------------------------------------
            // Send to video pipeline
            //
            // Don't block the capture thread if the network pipeline
            // falls behind. Dropping a frame is preferable to adding
            // latency.
            // ---------------------------------------------------------

            match video_tx.try_send(encoded) {
                Ok(()) => {}

                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    // Pipeline is behind.
                    // Drop this frame rather than building latency.
                }

                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    eprintln!("video channel closed");
                    return;
                }
            }
        }
    });
}