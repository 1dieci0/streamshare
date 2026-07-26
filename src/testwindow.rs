use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use pixels::{Pixels, SurfaceTexture};
use scrap::{Capturer, Display};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::gui::Framework;

struct SharedFrame {
    data: Option<Vec<u8>>,
}


struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    framework: Option<Framework>,

    frame: Arc<Mutex<SharedFrame>>,

    gui_visible: bool,
    last_mouse_move: Instant,
}


impl App {

    fn new(frame: Arc<Mutex<SharedFrame>>) -> Self {
        Self {
            window: None,
            pixels: None,
            framework: None,

            frame,

            gui_visible: true,
            last_mouse_move: Instant::now(),
        }
    }


    fn draw_video(&mut self) {

        let Some(pixels) = self.pixels.as_mut()
        else {
            return;
        };


        let Some(frame) = self.frame.lock()
            .unwrap()
            .data
            .clone()
        else {
            return;
        };


        let output = pixels.frame_mut();


        for (src, dst) in frame
            .chunks_exact(4)
            .zip(output.chunks_exact_mut(4))
        {
            // scrap gives BGRA
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = 255;
        }
    }


    fn draw_gui(&self) {

        if !self.gui_visible {
            return;
        }


        // This is where egui drawing would go.
        //
        // Example:
        //
        // ui.button("Stop stream");
        //
        // ui.label("Connected");
    }
}



impl ApplicationHandler for App {


    fn resumed(&mut self, event_loop: &ActiveEventLoop) {


        let window = Arc::new(event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("StreamShare")
            )
            .unwrap()
        );


        let size = window.inner_size();


        let surface = SurfaceTexture::new(
            size.width,
            size.height,
            window.clone(),
        );


        let pixels = Pixels::new(
            size.width,
            size.height,
            surface,
        )
        .unwrap();

        let scale_factor = (window.scale_factor()) as f32;

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


        match event {


            WindowEvent::CloseRequested => {
                event_loop.exit();
            }



            WindowEvent::CursorMoved { .. } => {

                self.gui_visible = true;

                self.last_mouse_move =
                    Instant::now();
            }



            WindowEvent::RedrawRequested => {

                self.framework.prepare(&self.window);
                // video always updates

                self.draw_video();


                // GUI only if needed

                println!("{:?}", self.last_mouse_move.elapsed());
                if self.last_mouse_move.elapsed()
                    > Duration::from_secs(3)
                {
                    self.gui_visible = false;
                }


                self.draw_gui();



                if let Some(pixels) = self.pixels.as_mut() {
                    pixels.render().unwrap();
                }
            }



            _ => {}
        }
    }



    fn about_to_wait(
        &mut self,
        _event_loop: &ActiveEventLoop,
    ) {

        // video refresh rate

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}



fn start_capture(
    output: Arc<Mutex<SharedFrame>>
) {


    thread::spawn(move || {


        let display =
            Display::primary()
            .unwrap();


        let mut capturer =
            Capturer::new(display)
            .unwrap();



        loop {


            match capturer.frame() {


                Ok(frame) => {

                    let mut buffer =
                        output.lock().unwrap();


                    buffer.data =
                        Some(frame.to_vec());
                }



                Err(_) => {

                    thread::sleep(
                        Duration::from_millis(5)
                    );
                }
            }
        }
    });
}





pub fn megatest() {


    let frame =
        Arc::new(
            Mutex::new(
                SharedFrame {
                    data: None
                }
            )
        );


    start_capture(
        Arc::clone(&frame)
    );



    let event_loop =
        EventLoop::new()
        .unwrap();



    let mut app =
        App::new(frame);



    event_loop
        .run_app(&mut app)
        .unwrap();
}