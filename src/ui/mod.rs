use tokio::sync::mpsc::{Receiver, Sender};

use crate::client::command::{ClientCommand, ClientEvent};

pub mod window;
mod gui;

pub fn start(
    command_tx: Sender<ClientCommand>,
    event_rx: Receiver<ClientEvent>,
) {
    let event_loop = winit::event_loop::EventLoop::<()>::with_user_event()
        .build()
        .expect("failed to create event loop");

    let mut app = window::App::new(
        command_tx,
        event_rx,
    );

    event_loop
        .run_app(&mut app)
        .expect("failed to run UI");
}