#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Instant;

use crate::gui::Framework;

use crate::screencapture;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::event_loop::ControlFlow;
use winit::keyboard::KeyCode;
use winit::window::WindowAttributes;
use winit_input_helper::WinitInputHelper;


pub fn checkwindows() -> Result<(), Error> {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut input = WinitInputHelper::new();


    let mut screen = screencapture::Screen::new().unwrap();

    let window = {
        let size = LogicalSize::new(screen.width as u32, screen.height as u32);
        Arc::new(
            #[allow(deprecated)]
            event_loop
                .create_window(
                    WindowAttributes::new()
                        .with_title("Hello Pixels + egui")
                        .with_inner_size(size)
                        .with_min_inner_size(size),
                )
                .unwrap(),
        )
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(screen.width as u32, screen.height as u32, surface_texture)?
    };
    
    let mut framework: Option<Framework> = None;

    #[allow(deprecated)]
    let res = event_loop.run(|event, event_loop| {
        match event {
            Event::Resumed => {
                let window_size = window.inner_size();
                let scale_factor = (window.scale_factor()) as f32;
                framework = Some(Framework::new(
                    event_loop,
                    window_size.width,
                    window_size.height,
                    scale_factor,
                    &pixels,
                ));
                window.request_redraw();
            }
            Event::NewEvents(_) => input.step(),
            Event::AboutToWait => {
                input.end_step();
            }
            Event::DeviceEvent { event, .. } => {
                input.process_device_event(&event);
            }
            Event::WindowEvent { event, .. } => {
                let Some(framework) = &mut framework else {
                    return;
                };

                // Handle input events
                if input.process_window_event(&event) {
                    // Update the scale factor
                    if let Some(scale_factor) = input.scale_factor() {
                        framework.scale_factor(scale_factor);
                    }

                    // Resize the window
                    if let Some(size) = input.window_resized()
                        && size.width > 0
                        && size.height > 0
                    {
                        if let Err(err) = pixels.resize_surface(size.width, size.height) {
                            event_loop.exit();
                            return;
                        }
                        framework.resize(size.width, size.height);
                    }
                }

                match event {

                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {

                    match screen.current_frame() {
                        Ok(frame) => {
                            let output = pixels.frame_mut();


                            for (src, dst) in frame
                                .chunks_exact(4)
                                .zip(output.chunks_exact_mut(4))
                            {
                                // BGRA -> RGBA
                                dst[0] = src[2];
                                dst[1] = src[1];
                                dst[2] = src[0];
                                dst[3] = 255;
                            }


                            pixels.render().unwrap();
                        }

                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            return;
                        }

                        Err(e) => {
                            println!("capture error: {}", e);
                        }

                    }

                        // Prepare egui
                        framework.prepare(&window);
                         // Render everything together
                        let render_result =
                            pixels.render_with(|encoder, render_target, context| {
                                // Render the world texture
                                context.scaling_renderer.render(encoder, render_target);

                                // Render egui
                                framework.render(encoder, render_target, context);

                                Ok(())
                            });

                        // Basic error handling
                        if let Err(err) = render_result {
                            event_loop.exit();
                        }
                        
                        window.request_redraw();

                }
                    event => {
                        // Update egui inputs
                        framework.handle_event(&window, &event);
                    }
                }
            }
            _ => {}
        }
    });
    res.map_err(|e| Error::UserDefined(Box::new(e)))
}






                