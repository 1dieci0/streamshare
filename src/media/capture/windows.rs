use std::{
    io,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::{Receiver, Sender};

use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    graphics_capture_picker::GraphicsCapturePicker,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings,
        DrawBorderSettings, MinimumUpdateIntervalSettings,
        SecondaryWindowSettings, Settings,
    },
};

use crate::{client::command::EncoderCommand, media::{capture::ffmpeg_encoder::FFmpegEncoder, encoder::EncodedFrame, state::Media}};

pub struct WindowsCapture {
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,

    encoder: FFmpegEncoder,

    sequence: u64,
    frames: u64,
    last_report: Instant,
}

impl GraphicsCaptureApiHandler for WindowsCapture {
    type Flags = (
        Arc<Media>,
        Sender<EncodedFrame>,
        FFmpegEncoder,
    );

    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let (media, video_tx, encoder) = ctx.flags;

        Ok(Self {
            media,
            video_tx,
            encoder,
            sequence: 0,
            frames: 0,
            last_report: Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        if !self.media.is_streaming() {
            capture_control.stop();
            return Ok(());
        }

        let start = Instant::now();

        let width = frame.width();
        let height = frame.height();

        /*
         * Copy the captured frame into CPU memory.
         *
         * The capture is BGRA8, so we want:
         *
         * D3D11 texture
         *      ↓
         * CPU BGRA bytes
         *      ↓
         * ffmpeg.exe
         *      ↓
         * h264_nvenc
         */
        let buffer = frame
            .buffer()?
            .as_raw_buffer()
            .to_vec();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        match self
            .encoder
            .encode_bgra_frame(&buffer, timestamp, self.sequence)?
        {
            Some(encoded_frame) => {
                match self.video_tx.try_send(encoded_frame) {
                    Ok(()) => {}

                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        // Drop the frame if the network side is behind.
                        //
                        // For live streaming this is preferable to building
                        // an ever-growing queue.
                    }

                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        eprintln!("Video channel closed");
                        capture_control.stop();
                    }
                }
            }

            None => {
                // FFmpeg hasn't produced an output packet yet.
            }
        }

        self.sequence += 1;
        self.frames += 1;

        if self.last_report.elapsed() >= Duration::from_secs(1) {
            println!(
                "capture FPS: {} | callback: {:.2} ms | {}x{} | frame: {}",
                self.frames,
                start.elapsed().as_secs_f64() * 1000.0,
                width,
                height,
                self.sequence,
            );

            self.frames = 0;
            self.last_report = Instant::now();
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Windows capture session ended");
        Ok(())
    }
}

pub fn start_capture(
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,
    _encoder_rx: Receiver<EncoderCommand>,
) -> io::Result<()> {
    thread::spawn(move || {
        /*
         * Wait until streaming starts.
         */
        while !media.is_streaming() {
            thread::sleep(Duration::from_millis(50));
        }

        /*
         * Let the user choose what to capture.
         */
        let item = match GraphicsCapturePicker::pick_item() {
            Ok(Some(item)) => item,

            Ok(None) => {
                eprintln!("No capture target selected");
                return;
            }

            Err(e) => {
                eprintln!("Capture picker failed: {e}");
                return;
            }
        };

        /*
         * Get the initial dimensions.
         */
        let size = match item.size() {
            Ok(size) => size,

            Err(e) => {
                eprintln!("Failed to get capture size: {e}");
                return;
            }
        };

        let width = u32::try_from(size.0)
            .map_err(|_| format!("Invalid capture width: {}", size.0));

        let height = u32::try_from(size.1)
            .map_err(|_| format!("Invalid capture height: {}", size.1));

        let (width, height) = match (width, height) {
            (Ok(width), Ok(height)) => (width, height),

            (Err(e), _) => {
                eprintln!("{e}");
                return;
            }

            (_, Err(e)) => {
                eprintln!("{e}");
                return;
            }
        };

        println!(
            "Starting capture: {}x{} @ 60 FPS",
            width,
            height
        );

        /*
         * Start FFmpeg/NVENC.
         *
         * ffmpeg.exe should either:
         *
         * 1. be in PATH
         *
         * or
         *
         * 2. be next to StreamShare.exe.
         */
        let encoder = match FFmpegEncoder::new(
            "ffmpeg/ffmpeg.exe",
            width,
            height,
            60,
            5000,
        ) {
            Ok(encoder) => encoder,

            Err(e) => {
                eprintln!("Failed to start FFmpeg encoder: {e}");
                return;
            }
        };

        /*
         * Capture as BGRA8.
         *
         * This means the frame we receive from windows-capture
         * matches the format expected by FFmpeg:
         *
         * BGRA → h264_nvenc
         */
        let settings = Settings::new(
            item,

            CursorCaptureSettings::Default,

            DrawBorderSettings::Default,

            SecondaryWindowSettings::Default,

            MinimumUpdateIntervalSettings::Custom(
                Duration::from_micros(1),
            ),

            DirtyRegionSettings::Default,

            ColorFormat::Bgra8,

            (
                media,
                video_tx,
                encoder,
            ),
        );

        if let Err(e) = WindowsCapture::start(settings) {
            eprintln!("Windows capture failed: {e}");
        }
    });

    Ok(())
}