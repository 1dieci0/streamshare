use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use pixels::{Pixels, SurfaceTexture};
use scrap::{Capturer, Display};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::gui::Framework;

struct SharedFrame {
    data: Option<Vec<u8>>,
}

struct VideoSize {
    width: usize,
    height: usize,
}

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    framework: Option<Framework>,
    frame: Arc<Mutex<SharedFrame>>,
    video_size: Option<VideoSize>,
}

impl App {
    fn new(frame: Arc<Mutex<SharedFrame>>) -> Self {
        Self {
            window: None,
            pixels: None,
            framework: None,
            frame,
            video_size: None,
        }
    }

    fn draw_video(&mut self) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        let Some(window) = self.window.as_ref() else {
            return;
        };

        let Some(video_size) = &self.video_size else {
            return;
        };

        let Some(frame) = self.frame.lock().unwrap().data.clone() else {
            return;
        };

        let window_size = window.inner_size();
        let target_width = window_size.width as usize;
        let target_height = window_size.height as usize;
        let output = pixels.frame_mut();
        let row_bytes = video_size.width.saturating_mul(4);

        if row_bytes == 0 || video_size.height == 0 || target_width == 0 || target_height == 0 {
            return;
        }

        let src_stride = frame.len() / video_size.height;
        if src_stride < row_bytes {
            return;
        }

        for pixel in output.chunks_exact_mut(4) {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 255;
        }

        let scale_x = target_width as f32 / video_size.width as f32;
        let scale_y = target_height as f32 / video_size.height as f32;
        let scale = scale_x.min(scale_y);

        let scaled_width = (video_size.width as f32 * scale).round() as usize;
        let scaled_height = (video_size.height as f32 * scale).round() as usize;

        if scaled_width == 0 || scaled_height == 0 {
            return;
        }

        let offset_x = (target_width - scaled_width) / 2;
        let offset_y = (target_height - scaled_height) / 2;

        for dst_y in 0..scaled_height {
            let src_y = dst_y.saturating_mul(video_size.height) / scaled_height;
            let src_row_start = src_y.saturating_mul(src_stride);
            let src_row_end = src_row_start + row_bytes;

            if src_row_end > frame.len() {
                break;
            }

            let src_row = &frame[src_row_start..src_row_end];
            let dst_row_start = (dst_y + offset_y).saturating_mul(target_width).saturating_mul(4);

            for dst_x in 0..scaled_width {
                let src_x = dst_x.saturating_mul(video_size.width) / scaled_width;
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let display = Display::primary().unwrap();
        let size = PhysicalSize::new(display.width() as u32, display.height() as u32);

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
        self.video_size = Some(VideoSize {
            width: size.width as usize,
            height: size.height as usize,
        });
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
                event_loop.exit();
            }

            WindowEvent::CursorMoved { .. } => {
                if let Some(framework) = self.framework.as_mut() {
                    framework.mouse_moved();
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
                if let Some(framework) = self.framework.as_mut() {
                    framework.update();
                }

                self.draw_video();

                if let Some(framework) = self.framework.as_mut() {
                    if let Some(window) = self.window.as_ref() {
                        framework.prepare(window);
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn start_capture(output: Arc<Mutex<SharedFrame>>) {
    thread::spawn(move || {
        let display = Display::primary().unwrap();
        let mut capturer = Capturer::new(display).unwrap();

        loop {
            match capturer.frame() {
                Ok(frame) => {
                    let mut buffer = output.lock().unwrap();
                    buffer.data = Some(frame.to_vec());
                }

                Err(_) => {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
    });
}

pub fn megatest() {
    let frame = Arc::new(Mutex::new(SharedFrame { data: None }));

    start_capture(Arc::clone(&frame));

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(frame);

    event_loop.run_app(&mut app).unwrap();
}
