use std::sync::{Mutex, Arc, RwLock};

use winit::event_loop::EventLoop;

use crate::{client::{state::ClientState, ui::state::AppState}, media::{frame::SharedFrame, state::MediaState}};

mod window;
mod gui;
pub mod state;


pub fn start_ui(
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,
) {
    let frame = Arc::new(Mutex::new(SharedFrame { data: None }));

    window::start_capture(Arc::clone(&frame));

    let event_loop = EventLoop::new().unwrap();
    let mut app = window::App::new(frame, app_state);

    event_loop.run_app(&mut app).unwrap();
}