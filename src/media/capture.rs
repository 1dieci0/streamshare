use std::{
    io,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use scrap::{Capturer, Display};
use tokio::sync::mpsc::Sender;

use crate::media::{
    encoder::{EncodedFrame, VideoEncoder},
    frame::RawFrame,
    state::Media,
};

pub struct Screen {
    capturer: Capturer,
    pub height: usize,
    pub width: usize,
}

impl Screen {
    pub fn new() -> io::Result<Self> {
        let display = Display::primary()?;

        let width = display.width();
        let height = display.height();

        let capturer = Capturer::new(display)?;

        Ok(Self {
            capturer,
            height,
            width,
        })
    }

    pub fn current_frame(&mut self) -> io::Result<RawFrame> {
        let frame = self.capturer.frame()?;

        let row_bytes = frame.len() / self.height;
        let packed_bytes = self.width * 4;

        if row_bytes < packed_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "screen capture frame shorter than expected",
            ));
        }

        let mut data = vec![0u8; self.width * self.height * 4];

        for row in 0..self.height {
            let src_start = row * row_bytes;
            let src_end = src_start + packed_bytes;

            let dst_start = row * packed_bytes;
            let dst_end = dst_start + packed_bytes;

            data[dst_start..dst_end]
                .copy_from_slice(&frame[src_start..src_end]);
        }

        Ok(RawFrame {
            sequence: 0,
            timestamp: 0,
            width: self.width,
            height: self.height,
            data,
        })
    }
}

pub fn start_capture(
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,
) {
    thread::spawn(move || {
        let Ok(mut screen) = Screen::new() else {
            eprintln!("Failed to initialize screen capture");
            return;
        };

        let mut encoder = match VideoEncoder::new() {
            Ok(encoder) => encoder,

            Err(e) => {
                eprintln!("Failed to initialize H264 encoder: {e}");
                return;
            }
        };

        let mut sequence = 0u64;

        let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

        let mut next_frame = Instant::now();
        let mut stream_start = Instant::now();

        loop {
            // ---------------------------------------------------------
            // Not streaming
            // ---------------------------------------------------------

            if !media.is_streaming() {
                thread::sleep(Duration::from_millis(50));

                next_frame = Instant::now();
                stream_start = Instant::now();
                sequence = 0;

                continue;
            }

            // ---------------------------------------------------------
            // Schedule next frame
            // ---------------------------------------------------------

            next_frame += frame_duration;

            // ---------------------------------------------------------
            // Capture
            // ---------------------------------------------------------

            match screen.current_frame() {
                Ok(mut frame) => {
                    frame.sequence = sequence;

                    frame.timestamp =
                        stream_start.elapsed().as_micros() as u64;

                    sequence += 1;

                    // -------------------------------------------------
                    // H264 encode
                    // -------------------------------------------------

                    let encoded = match encoder.encode_frame(&frame) {
                        Ok(encoded) => encoded,

                        Err(e) => {
                            eprintln!("H264 encoding error: {e}");
                            continue;
                        }
                    };

                    println!(
                        "ENCODED frame={} bytes={} keyframe={}",
                        encoded.sequence,
                        encoded.data.len(),
                        encoded.keyframe
                    );

                    // -------------------------------------------------
                    // Send to video pipeline
                    //
                    // IMPORTANT:
                    // try_send() means we never wait for the network.
                    //
                    // If the receiver is busy, the frame is dropped.
                    // -------------------------------------------------

                    match video_tx.try_send(encoded) {
                        Ok(()) => {}

                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                            // Receiver is behind.
                            //
                            // Drop this encoded frame rather than
                            // increasing latency.
                        }

                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            eprintln!("video channel closed");
                            return;
                        }
                    }
                }

                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // scrap has no frame ready yet.
                }

                Err(e) => {
                    eprintln!("capture error: {e}");
                }
            }

            // ---------------------------------------------------------
            // Maintain 60 FPS
            // ---------------------------------------------------------

            let now = Instant::now();

            if next_frame > now {
                thread::sleep(next_frame - now);
            } else {
                // We're behind.
                //
                // Don't process a backlog. Skip directly to "now".
                next_frame = now;
            }
        }
    });
}