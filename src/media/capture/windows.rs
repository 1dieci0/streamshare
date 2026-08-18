use std::{
    io, sync::Arc, thread, time::{Duration, Instant},
};

use tokio::sync::mpsc::{Receiver, Sender};

use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    graphics_capture_picker::GraphicsCapturePicker,
    settings::{
        ColorFormat,
        CursorCaptureSettings,
        DirtyRegionSettings,
        DrawBorderSettings,
        MinimumUpdateIntervalSettings,
        SecondaryWindowSettings,
        Settings,
    },
};

use crate::{
    client::command::EncoderCommand, media::{
        encoder::{self, EncodedFrame, VideoEncoder}, frame::RawFrame, state::Media,
    },
};

pub struct WindowsCapture {
    media: Arc<Media>,
    video_tx: Sender<EncodedFrame>,
    encoder: VideoEncoder,
    sequence: u64,

    frames: u64,
    last_report : Instant,
}

impl GraphicsCaptureApiHandler for WindowsCapture {
    type Flags = (
        Arc<Media>,
        Sender<EncodedFrame>,
        VideoEncoder,
    );

    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(
        ctx: Context<Self::Flags>,
    ) -> Result<Self, Self::Error> {
        let (media, video_tx, encoder) = ctx.flags;

        //let encoder = VideoEncoder::new(encoder_rx)?;

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

        // If streaming was stopped while the capture session
        // was running, stop this Windows Capture session.
        if !self.media.is_streaming() {
            capture_control.stop();
            return Ok(());
        }

        let now = Instant::now();


        let mut buffer = frame.buffer()?;

        let width = buffer.width() as usize;
        let height = buffer.height() as usize;

        let mut storage = Vec::with_capacity(width * height * 4);

        let data = buffer.as_nopadding_buffer(&mut storage);

        let raw_frame = RawFrame {
            sequence: self.sequence,
            timestamp: 0,
            width,
            height,
            data: data.to_vec(),
        };

        self.sequence += 1;

        // // Your existing OpenH264 encoder.
        // let encoded = self.encoder.encode_frame(&raw_frame)?;

        // if encoded.data.is_empty() {
        //     return Ok(());
        // }

        // Don't block the Windows capture callback.
        // match self.video_tx.try_send(encoded) {
        //     Ok(()) => {}

        //     Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
        //         // Encoder/network pipeline is behind.
        //         // Drop the frame rather than increasing latency.
        //     }

        //     Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
        //         capture_control.stop();
        //     }
        // }

        let elapsted = now.elapsed();
        self.frames += 1;
            if self.last_report.elapsed() >= Duration::from_secs(1) {
        println!(
            "capture FPS: {} | processing: {:.2} ms | {}x{}",
            self.frames,
            elapsted.as_secs_f64() * 1000.0,
            width,
            height,
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
    encoder_rx: Receiver<EncoderCommand>,
) -> io::Result<()> {
    thread::spawn(move || {
        // Wait until streaming starts.
        while !media.is_streaming() {
            thread::sleep(
                std::time::Duration::from_millis(50)
            );
        }

        let mut encoder = match VideoEncoder::new(encoder_rx) {
            Ok(encoder) => encoder,
            Err(e) => {
                eprintln!("failed to create encoder: {e}");
                return;
            }
        };

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

        let size = match item.size() {
            Ok(size) => size,

            Err(e) => {
                eprintln!("Failed to get capture size: {e}");
                return;
            }
        };

        println!(
            "Starting capture {}x{}",
            size.0,
            size.1
        );

        let settings = Settings::new(
            item,

            CursorCaptureSettings::Default,
            DrawBorderSettings::Default,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Custom(Duration::from_micros(1)),
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,

            (
                media,
                video_tx,
                encoder, 
            ),
        );

        if let Err(e) =
            WindowsCapture::start(settings)
        {
            eprintln!(
                "Windows capture failed: {e}"
            );
        }
    });

    Ok(())
}