use std::{
    sync::{Arc, RwLock}, time::{Duration, Instant},

};

use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop},
    window::{Window, WindowId},
};

use crate::{client::{state::{ClientCommand, ClientState}, ui::{event::AppEvent, gui::Framework, state::AppState}}, media::state::MediaState};



pub struct App {
    client_state: Arc<ClientState>,
    app_state: Arc<RwLock<AppState>>,
    media_state: Arc<MediaState>,

    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    framework: Option<Framework>,

    last_ui_update: Instant,
    ui_dirty: bool,
}

impl App {
    pub fn new(
        client_state: Arc<ClientState>,
        app_state: Arc<RwLock<AppState>>,
        media_state: Arc<MediaState>,
    ) -> Self {
        Self {
            client_state,
            app_state,
            media_state,
            window: None,
            pixels: None,
            framework: None,

            last_ui_update: Instant::now(),
            ui_dirty: true,
        }
    }

    fn draw_video(&mut self) {

        let selected_uid = {
            let state = self.app_state.read().unwrap();
            state.selected_stream
        };

        let Some(uid) = selected_uid else {
            return;
        };


        let Some(frame) = self.media_state.incoming(uid) else {
            eprintln!("{uid} is not streaming");
            return;
        };



        let video_width = frame.width;
        let video_height = frame.height;
        let frame = &frame.data;




        let Some(pixels) = self.pixels.as_mut() else {
            eprintln!("Pixels error while drawing stream");
            return;
        };

        let Some(window) = self.window.as_ref() else {
            eprintln!("Window error while drawing stream");
            return;
        };

        let window_size = window.inner_size();
        let target_width = window_size.width as usize;
        let target_height = window_size.height as usize;
        let output = pixels.frame_mut();
        let row_bytes = video_width.saturating_mul(4);

        if row_bytes == 0 || video_height == 0 || target_width == 0 || target_height == 0 {
            return;
        }

        let src_stride = frame.len() / video_height;
        if src_stride < row_bytes {
            return;
        }

        for pixel in output.chunks_exact_mut(4) {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 255;
        }

        let scale_x = target_width as f32 /video_width as f32;
        let scale_y = target_height as f32 / video_height as f32;
        let scale = scale_x.min(scale_y);

        let scaled_width = (video_width as f32 * scale).round() as usize;
        let scaled_height = (video_height as f32 * scale).round() as usize;

        if scaled_width == 0 || scaled_height == 0 {
            return;
        }

        let offset_x = (target_width - scaled_width) / 2;
        let offset_y = (target_height - scaled_height) / 2;

        for dst_y in 0..scaled_height {
            let src_y = dst_y.saturating_mul(video_height) / scaled_height;
            let src_row_start = src_y.saturating_mul(src_stride);
            let src_row_end = src_row_start + row_bytes;

            if src_row_end > frame.len() {
                break;
            }

            let src_row = &frame[src_row_start..src_row_end];
            let dst_row_start = (dst_y + offset_y).saturating_mul(target_width).saturating_mul(4);

            for dst_x in 0..scaled_width {
                let src_x = dst_x.saturating_mul(video_width) / scaled_width;
                let src_index = src_x.saturating_mul(4);
                let dst_index = dst_row_start + (dst_x + offset_x).saturating_mul(4);

                if src_index + 3 >= src_row.len() || dst_index + 3 >= output.len() {
                    continue;
                }

                // scrap gives BGRA
                output[dst_index] = src_row[src_index + 2];
                output[dst_index + 1] = src_row[src_index + 1];
                output[dst_index + 2] = src_row[src_index];
                output[dst_index + 3] = 255;
            }
        }

    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let size = PhysicalSize::new(1280, 720);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("StreamShare")
                        .with_inner_size(size),
                )
                .unwrap(),
        );

        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = Pixels::new(size.width, size.height, surface).unwrap();

        let scale_factor = window.scale_factor() as f32;
        let framework = Framework::new(
            event_loop,
            size.width,
            size.height,
            scale_factor,
            &pixels,
        );

        self.window = Some(window);
        self.pixels = Some(pixels);
        self.framework = Some(framework);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(framework) = self.framework.as_mut() {
            if let Some(window) = self.window.as_ref() {
                framework.handle_event(window, &event);
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.client_state.set_command(ClientCommand::Disconnect);
                event_loop.exit();
            }

            WindowEvent::CursorMoved { .. } => {
                self.ui_dirty = true;

                if let Some(framework)=self.framework.as_mut() {
                    framework.mouse_moved();
                }

                if let Some(window)=&self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    if pixels.resize_surface(size.width, size.height).is_err() {
                        event_loop.exit();
                        return;
                    }

                    if pixels.resize_buffer(size.width, size.height).is_err() {
                        event_loop.exit();
                        return;
                    }
                }

                if let Some(framework) = self.framework.as_mut() {
                    framework.resize(size.width, size.height);
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(framework) = self.framework.as_mut() {
                    framework.scale_factor(scale_factor);
                }
            }

            WindowEvent::RedrawRequested => {
                let update_ui = self.ui_dirty 
                    || self.last_ui_update.elapsed() >= Duration::from_millis(100);

                if update_ui {
                    if let Some(framework) = self.framework.as_mut() {
                        framework.update();
                    }

                    self.last_ui_update = Instant::now();
                    self.ui_dirty = false;
                }

                self.draw_video();

                if update_ui {
                    if let Some(framework) = self.framework.as_mut() {
                        if let Some(window) = self.window.as_ref() {
                            framework.prepare(
                                window,
                                Arc::clone(&self.app_state),
                                Arc::clone(&self.client_state),
                                Arc::clone(&self.media_state),
                            );
                        }
                    }
                }

                if let Some(pixels) = self.pixels.as_mut() {
                    let Some(framework) = self.framework.as_mut() else {
                        return;
                    };

                    let render_result = pixels.render_with(|encoder, render_target, context| {
                        context.scaling_renderer.render(encoder, render_target);
                        framework.render(encoder, render_target, context);
                        Ok(())
                    });

                    if render_result.is_err() {
                        event_loop.exit();
                    }
                }
            }

            _ => {}
        }
    }

    fn user_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        event: AppEvent,
    ) {

        match event {

            AppEvent::NewFrame(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            AppEvent::UserJoined(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            AppEvent::UserLeft(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }


            AppEvent::StreamStarted(_)=>{
                if let Some(window)=&self.window {
                    window.request_redraw();
                }
            }

            AppEvent::StreamStopped(_)=>{
                if let Some(window)=&self.window {
                    window.request_redraw();
                }
            }

            _=>{}

        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };

        // Keep egui alive at 10 FPS
        if self.last_ui_update.elapsed() >= Duration::from_millis(100) {
            window.request_redraw();
        }

        // egui requested an immediate repaint
        if let Some(framework) = &self.framework {
            if framework.needs_repaint() {
                window.request_redraw();
            }
        }
    }
}
