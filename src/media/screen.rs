use scrap::{Capturer, Display, Frame};
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

    pub fn current_frame(&mut self) -> io::Result<Frame<'_>> {
        self.capturer.frame()
    }
}