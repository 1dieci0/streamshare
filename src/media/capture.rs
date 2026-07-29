use std::{sync::{Arc, atomic::Ordering}, thread, time::Duration};

use crate::{client::state::ClientState, media::{screen::Screen, state::MediaState}};

pub fn start_capture(
    media: Arc<MediaState>,
    client: Arc<ClientState>,
) {
    thread::spawn(move || {
        let mut screen = Screen::new().unwrap();

        loop {
            if !client.streaming.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            if let Ok(frame) = screen.current_frame() {
                media.update_capture(...);
            }
        }
    });
}