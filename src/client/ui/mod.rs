use std::sync::{Arc, RwLock};

use winit::event_loop::EventLoop;

use crate::{client::{state::ClientState, ui::{state::AppState}}, media::state::MediaState};

pub mod window;
mod gui;
pub mod state;
pub mod event;


pub fn start_ui(
    event_loop: EventLoop<event::AppEvent>,
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,
) {

    let mut app = window::App::new(
        client_state,
        app_state,
        media_state,
    );

    event_loop
        .run_app(&mut app)
        .unwrap();
}