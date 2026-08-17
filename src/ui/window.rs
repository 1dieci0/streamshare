use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use pixels::{Pixels, SurfaceTexture};
use tokio::sync::mpsc::{Receiver, Sender};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{
    client::command::{ClientCommand, ClientEvent}, media::frame::RawFrame, protocol::video::VideoPacket, ui::gui::Framework,
};

pub struct App {
    // Communication with the client/network layer.
    command_tx: Sender<ClientCommand>,
    event_rx: Receiver<ClientEvent>,
    video_rx: Receiver<RawFrame>,

    // UI-owned state.
    users: HashMap<u64, String>,
    streams: HashMap<u64, String>,

    // Latest frame received for each stream.
    latest_frame: Option<RawFrame>,

    // Currently selected stream.
    selected_stream: Option<u64>,

    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    framework: Option<Framework>,

    last_ui_update: Instant,
    ui_dirty: bool,
}

impl App {
    pub fn new(
        command_tx: Sender<ClientCommand>,
        event_rx: Receiver<ClientEvent>,
        mut video_rx: Receiver<RawFrame>,
    ) -> Self {
        Self {
            command_tx,
            event_rx,
            video_rx,

            users: HashMap::new(),
            streams: HashMap::new(),
            latest_frame: None,

            selected_stream: None,

            window: None,
            pixels: None,
            framework: None,

            last_ui_update: Instant::now(),
            ui_dirty: true,
        }
    }

    fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_client_event(event);
        }
    }

    fn handle_client_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Connected => {
                println!("connected");

                self.ui_dirty = true;
            }

            ClientEvent::Authenticated{uid} => {
                println!("authenticated with uid {uid}");

                self.ui_dirty = true;
            }

            ClientEvent::UserJoined {
                uid,
                username,
            } => {
                self.users.insert(uid, username);

                self.ui_dirty = true;
            }

            ClientEvent::UserLeft {
                uid,
                ..
            } => {
                self.users.remove(&uid);
                self.streams.remove(&uid);
                self.latest_frame = None;

                if self.selected_stream == Some(uid) {
                    self.selected_stream = None;
                }

                self.ui_dirty = true;
            }

            ClientEvent::StreamStarted {
                uid,
                username,
            } => {
                self.streams.insert(uid, username);

                self.ui_dirty = true;
            }

            ClientEvent::StreamStopped {
                uid,
                ..
            } => {
                self.streams.remove(&uid);
                self.latest_frame = None;

                if self.selected_stream == Some(uid) {
                    self.selected_stream = None;
                }

                self.ui_dirty = true;
            }

            ClientEvent::InitialState { users, streams } => {
                for user in users{
                    self.users.insert(user.uid, user.username);
                }

                for stream in streams{
                    self.streams.insert(stream.uid, stream.username);
                }
                
                self.ui_dirty = true;
            }

            ClientEvent::Error(error) => {
                eprintln!("network error: {error}");

                self.ui_dirty = true;
            }

            ClientEvent::Disconnected => {
                eprintln!("disconnected");

                self.ui_dirty = true;
            }

            ClientEvent::WatchStream { uid, username, stream_uid, stream_username } => {
                
            }
        }
    }

    fn handle_command(&mut self, command: ClientCommand) {
        let command_tx = self.command_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = command_tx.send(command).await {
                eprintln!("failed to send client command: {e}");
            }
        });
    }

    fn draw_video(&mut self) {
        // let Some(uid) = self.selected_stream else {
        //     return;
        // };

        let Some(frame) = self.latest_frame.as_ref() else{
            return;
        };

        let video_width = frame.width as usize;
        let video_height = frame.height as usize;
        let frame_data = &frame.data;

        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let Some(window) = self.window.as_ref() else {
            return;
        };

        let window_size = window.inner_size();

        let target_width = window_size.width as usize;
        let target_height = window_size.height as usize;

        if video_width == 0
            || video_height == 0
            || target_width == 0
            || target_height == 0
        {
            return;
        }

        let row_bytes = video_width.saturating_mul(4);

        if row_bytes == 0 || frame_data.len() < row_bytes {
            return;
        }

        let src_stride = frame_data.len() / video_height;

        if src_stride < row_bytes {
            return;
        }

        let output = pixels.frame_mut();

        // Clear screen.
        for pixel in output.chunks_exact_mut(4) {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 255;
        }

        let scale_x = target_width as f32 / video_width as f32;
        let scale_y = target_height as f32 / video_height as f32;

        let scale = scale_x.min(scale_y);

        let scaled_width =
            (video_width as f32 * scale).round() as usize;

        let scaled_height =
            (video_height as f32 * scale).round() as usize;

        if scaled_width == 0 || scaled_height == 0 {
            return;
        }

        let offset_x =
            (target_width - scaled_width) / 2;

        let offset_y =
            (target_height - scaled_height) / 2;

        for dst_y in 0..scaled_height {
            let src_y =
                dst_y.saturating_mul(video_height)
                    / scaled_height;

            let src_row_start =
                src_y.saturating_mul(src_stride);

            let src_row_end =
                src_row_start + row_bytes;

            if src_row_end > frame_data.len() {
                break;
            }

            let src_row =
                &frame_data[src_row_start..src_row_end];

            let dst_row_start =
                (dst_y + offset_y)
                    .saturating_mul(target_width)
                    .saturating_mul(4);

            for dst_x in 0..scaled_width {
                let src_x =
                    dst_x.saturating_mul(video_width)
                        / scaled_width;

                let src_index =
                    src_x.saturating_mul(4);

                let dst_index =
                    dst_row_start
                        + (dst_x + offset_x)
                            .saturating_mul(4);

                if src_index + 3 >= src_row.len()
                    || dst_index + 3 >= output.len()
                {
                    continue;
                }

                // SCRAP gives BGRA.
                output[dst_index] =
                    src_row[src_index + 2];

                output[dst_index + 1] =
                    src_row[src_index + 1];

                output[dst_index + 2] =
                    src_row[src_index];

                output[dst_index + 3] = 255;
            }
        }
    }

    fn request_watch_stream(&mut self, uid: u64) {
        self.selected_stream = Some(uid);

        self.handle_command(
            ClientCommand::WatchStream { uid }
        );

        self.ui_dirty = true;
    }

    fn start_stream(&mut self) {
        self.handle_command(
            ClientCommand::StartStream
        );
    }

    fn stop_stream(&mut self) {
        self.handle_command(
            ClientCommand::StopStream
        );
    }

    fn disconnect(&mut self) {
        self.handle_command(
            ClientCommand::Disconnect
        );
    }


    fn process_video(&mut self) {
        while let Ok(frame) = self.video_rx.try_recv() {
            self.latest_frame = Some(frame);

            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) {
        let size = PhysicalSize::new(1280, 720);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("StreamShare")
                        .with_inner_size(size),
                )
                .expect("failed to create window"),
        );

        let surface = SurfaceTexture::new(
            size.width,
            size.height,
            window.clone(),
        );

        let pixels =
            Pixels::new(
                size.width,
                size.height,
                surface,
            )
            .expect("failed to create pixels");

        let scale_factor =
            window.scale_factor() as f32;

        let framework = Framework::new(
            event_loop,
            size.width,
            size.height,
            scale_factor,
            &pixels,
            self.command_tx.clone(),
        );

        self.window = Some(window);
        self.pixels = Some(pixels);
        self.framework = Some(framework);

        self.ui_dirty = true;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(framework) = self.framework.as_mut() {
            if let Some(window) = self.window.as_ref() {
                framework.handle_event(
                    window,
                    &event,
                );
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.disconnect();

                event_loop.exit();
            }

            WindowEvent::CursorMoved { .. } => {
                self.ui_dirty = true;

                if let Some(framework) =
                    self.framework.as_mut()
                {
                    framework.mouse_moved();
                }

                if let Some(window) =
                    &self.window
                {
                    window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(pixels) =
                    self.pixels.as_mut()
                {
                    if pixels
                        .resize_surface(
                            size.width,
                            size.height,
                        )
                        .is_err()
                    {
                        event_loop.exit();
                        return;
                    }

                    if pixels
                        .resize_buffer(
                            size.width,
                            size.height,
                        )
                        .is_err()
                    {
                        event_loop.exit();
                        return;
                    }
                }

                if let Some(framework) =
                    self.framework.as_mut()
                {
                    framework.resize(
                        size.width,
                        size.height,
                    );
                }
            }

            WindowEvent::ScaleFactorChanged {
                scale_factor,
                ..
            } => {
                if let Some(framework) =
                    self.framework.as_mut()
                {
                    framework.scale_factor(
                        scale_factor
                    );
                }
            }

            WindowEvent::RedrawRequested => {
                let update_ui =
                    self.ui_dirty
                        || self.last_ui_update.elapsed()
                            >= Duration::from_millis(100);

                if update_ui {
                    if let Some(framework) =
                        self.framework.as_mut()
                    {
                        framework.update();
                        framework.set_state(
                            self.users.clone(),
                            self.streams.clone(),
                            self.selected_stream,
                        );
                    }

                    self.last_ui_update =
                        Instant::now();

                    self.ui_dirty = false;
                }

                self.draw_video();

                if update_ui {
                    if let Some(framework) =
                        self.framework.as_mut()
                    {
                        if let Some(window) =
                            self.window.as_ref()
                        {
                            framework.prepare(
                                window,
                            );
                        }
                    }
                }

                if let Some(pixels) =
                    self.pixels.as_mut()
                {
                    let Some(framework) =
                        self.framework.as_mut()
                    else {
                        return;
                    };

                    let render_result =
                        pixels.render_with(
                            |encoder,
                             render_target,
                             context| {
                                context
                                    .scaling_renderer
                                    .render(
                                        encoder,
                                        render_target,
                                    );

                                framework.render(
                                    encoder,
                                    render_target,
                                    context,
                                );

                                Ok(())
                            },
                        );

                    if render_result.is_err() {
                        event_loop.exit();
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(
        &mut self,
        _event_loop: &ActiveEventLoop,
    ) {
        // Network events are delivered through Tokio's
        // channel. Drain everything currently available.
        self.process_events();
        self.process_video();

        let Some(window) = &self.window else {
            return;
        };

        if self.ui_dirty
            || self.last_ui_update.elapsed()
                >= Duration::from_millis(100)
        {
            window.request_redraw();
        }

        if let Some(framework) =
            &self.framework
        {
            if framework.needs_repaint() {
                window.request_redraw();
            }
        }
    }
}