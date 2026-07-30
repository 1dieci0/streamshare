use std::{sync::{Arc, atomic::Ordering}, thread, time::Duration};

use crate::{client::state::ClientState, media::{frame::SharedFrame, state::MediaState}};
use scrap::{Capturer, Display};
use std::io;

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

    pub fn current_frame(&mut self) -> io::Result<SharedFrame> {
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

            data[dst_start..dst_end].copy_from_slice(&frame[src_start..src_end]);
        }

        Ok(SharedFrame{
            sequence: 0,
            width: self.width,
            height: self.height,
            data,
        })
    }
}

pub fn start_capture(
    media_state: Arc<MediaState>,
    client_state: Arc<ClientState>,
) {
    thread::spawn(move || {
        let Ok(mut screen) = Screen::new() else {
            eprintln!("Failed to initialize screen capture");
            return;
        };

        let mut sequence = 0u64;

        loop {
            if !client_state.streaming.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            if let Ok(mut frame) = screen.current_frame() {
                frame.sequence = sequence;
                sequence += 1;

                media_state.update_capture(frame);
            }
        }
    });
}



