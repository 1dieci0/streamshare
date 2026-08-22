use std::{
    io,
    sync::Arc,
    time::Instant,
};

use tokio::sync::mpsc::Sender;

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
    media::{
        frame::RawFrame,
        state::Media,
    },
};

pub struct WindowsCapture {
    media: Arc<Media>,
    frame_tx: Sender<RawFrame>,
    sequence: u64,
    stream_start: Instant,
}

impl GraphicsCaptureApiHandler for WindowsCapture {
    type Flags = (
        Arc<Media>,
        Sender<RawFrame>,
    );

    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            media: ctx.flags.0.clone(),
            frame_tx: ctx.flags.1.clone(),
            sequence: 0,
            stream_start: Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let callback_start = Instant::now();

        if !self.media.is_streaming() {
            println!("[CAPTURE] streaming stopped -> stopping capture");
            capture_control.stop();
            return Ok(());
        }

        // -------------------------------------------------------------
        // Get buffer
        // -------------------------------------------------------------

        let buffer_start = Instant::now();

        let mut buffer = frame.buffer()?;

        let width = buffer.width() as usize;
        let height = buffer.height() as usize;

        let buffer_time = buffer_start.elapsed();

        // -------------------------------------------------------------
        // Copy frame
        // -------------------------------------------------------------

        let copy_start = Instant::now();

        let mut storage =
            Vec::with_capacity(width * height * 4);

        let data =
            buffer.as_nopadding_buffer(&mut storage);

        let data = data.to_vec();

        let copy_time = copy_start.elapsed();

        // -------------------------------------------------------------
        // Create RawFrame
        // -------------------------------------------------------------

        let raw_frame = RawFrame {
            sequence: self.sequence,
            timestamp: self.stream_start.elapsed().as_micros() as u64,
            width,
            height,
            data,
        };

        self.sequence += 1;

        // -------------------------------------------------------------
        // Send to encoder
        // -------------------------------------------------------------

        let send_start = Instant::now();

        match self.frame_tx.try_send(raw_frame) {
            Ok(()) => {
                let send_time = send_start.elapsed();

                // println!(
                //     "[CAPTURE] frame={} {}x{} | buffer={:?} copy={:?} send={:?} total={:?}",
                //     self.sequence - 1,
                //     width,
                //     height,
                //     buffer_time,
                //     copy_time,
                //     send_time,
                //     callback_start.elapsed(),
                // );
            }

            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                println!(
                    "[CAPTURE] frame={} DROPPED - encoder queue full | copy={:?} total={:?}",
                    self.sequence - 1,
                    copy_time,
                    callback_start.elapsed(),
                );
            }

            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                println!("[CAPTURE] encoder channel closed");
                capture_control.stop();
            }
        }

        Ok(())
    }

    fn on_closed(
        &mut self,
    ) -> Result<(), Self::Error> {

        println!("Windows capture session closed");

        Ok(())
    }
}


// =====================================================================
// START CAPTURE
// =====================================================================

pub fn start_capture(
    media: Arc<Media>,
    frame_tx: Sender<RawFrame>,
) -> io::Result<()> {
    std::thread::spawn(move || {
        loop {
            // =========================================================
            // WAIT UNTIL STREAMING STARTS
            // =========================================================

            while !media.is_streaming() {
                std::thread::sleep(
                    std::time::Duration::from_millis(50)
                );
            }

            println!("Starting Windows capture");

            // =========================================================
            // PICK CAPTURE TARGET
            // =========================================================

            let item = match GraphicsCapturePicker::pick_item() {
                Ok(Some(item)) => item,

                Ok(None) => {
                    eprintln!("No capture target selected");

                    // Don't busy-loop if picker was cancelled.
                    std::thread::sleep(
                        std::time::Duration::from_millis(500)
                    );

                    continue;
                }

                Err(e) => {
                    eprintln!("Capture picker failed: {e}");

                    std::thread::sleep(
                        std::time::Duration::from_secs(1)
                    );

                    continue;
                }
            };

            let size = match item.size() {
                Ok(size) => size,

                Err(e) => {
                    eprintln!("Failed to get capture size: {e}");
                    continue;
                }
            };

            println!(
                "Capturing {}x{}",
                size.0,
                size.1
            );

            // =========================================================
            // CREATE SESSION
            // =========================================================

            let settings = Settings::new(
                item,
                CursorCaptureSettings::Default,
                DrawBorderSettings::Default,
                SecondaryWindowSettings::Default,

                MinimumUpdateIntervalSettings::Custom(
                    std::time::Duration::from_millis(1)
                ),

                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,

                (media.clone(), frame_tx.clone()),
            );

            // =========================================================
            // BLOCK HERE WHILE CAPTURING
            // =========================================================

            if let Err(e) =
                WindowsCapture::start(settings)
            {
                eprintln!(
                    "Windows capture failed: {e}"
                );
            }

            println!("Windows capture stopped");

            // =========================================================
            // NOW WE ARE BACK HERE AFTER STOP STREAMING
            //
            // Loop goes back to:
            //
            // while !media.is_streaming()
            //
            // =========================================================
        }
    });

    Ok(())
}