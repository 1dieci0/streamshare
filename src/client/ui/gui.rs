use std::{sync::{Arc, RwLock}, time::{Duration, Instant}};

use egui::{ClippedPrimitive, Context, TexturesDelta, ViewportId};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use pixels::{PixelsContext, wgpu};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{client, media};
use crate::client::state::ClientCommand;

/// Manages all state required for rendering egui over `Pixels`.
pub(crate) struct Framework {
    // State for egui.
    egui_ctx: Context,
    egui_state: egui_winit::State,
    screen_descriptor: ScreenDescriptor,
    renderer: Renderer,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,

    // State for the GUI
    gui: Gui,
}

/// Example application state. A real application will need a lot more state than this.
struct Gui {

    visible: bool,
    last_mouse_move: Instant,

    notifications_open: bool,

    //users: Vec<String>,
}

impl Framework {
    /// Create egui.
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        width: u32,
        height: u32,
        scale_factor: f32,
        pixels: &pixels::Pixels,
    ) -> Self {
        let max_texture_size = pixels.device().limits().max_texture_dimension_2d as usize;

        let egui_ctx = Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(scale_factor),
            None,
            Some(max_texture_size),
        );
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };
        let renderer = Renderer::new(
            pixels.device(),
            pixels.render_texture_format(),
            RendererOptions::default(),
        );
        let textures = TexturesDelta::default();
        let gui = Gui::new();

        Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            renderer,
            paint_jobs: Vec::new(),
            textures,
            gui,
        }
    }

    /// Handle input events from the window manager.
    pub(crate) fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) {
        let _ = self.egui_state.on_window_event(window, event);
    }

    /// Resize egui.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.screen_descriptor.size_in_pixels = [width, height];
        }
    }

    /// Update scaling factor.
    pub(crate) fn scale_factor(&mut self, scale_factor: f64) {
        self.screen_descriptor.pixels_per_point = scale_factor as f32;
    }

    /// Prepare egui.
    pub(crate) fn prepare(
        &mut self,
        window: &Window,
        app_state: Arc<RwLock<client::ui::state::AppState>>,
        client_state: Arc<client::state::ClientState>,
        media_state: Arc<media::state::MediaState>,
    ) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run_ui(raw_input, |ui| {
            // Draw the demo application.
            self.gui.ui(
                &self.egui_ctx,
                ui,
                app_state.clone(),
                client_state.clone(),
                media_state.clone(),
            );
        });

        self.textures.append(output.textures_delta);
        self.egui_state
            .handle_platform_output(window, output.platform_output);
        self.paint_jobs = self
            .egui_ctx
            .tessellate(output.shapes, self.screen_descriptor.pixels_per_point);
    }

    /// Render egui.
    pub(crate) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) {
        // Upload all resources to the GPU.
        for (id, image_delta) in &self.textures.set {
            self.renderer
                .update_texture(&context.device, &context.queue, *id, image_delta);
        }
        self.renderer.update_buffers(
            &context.device,
            &context.queue,
            encoder,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        // Render egui with WGPU
        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();

            self.renderer
                .render(&mut rpass, &self.paint_jobs, &self.screen_descriptor);
        }

        // Cleanup
        let textures = std::mem::take(&mut self.textures);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
    }

    pub fn mouse_moved(&mut self) {
        self.gui.visible = true;
        self.gui.last_mouse_move = Instant::now();
    }

    pub fn update(&mut self) {
        if self.gui.last_mouse_move.elapsed() > Duration::from_secs(3) {
            self.gui.visible = false;
        }
    }
    
    pub fn needs_repaint(&self) -> bool {
        self.egui_ctx.has_requested_repaint()
    }
}

impl Gui {
    /// Create a `Gui`.
    fn new() -> Self {
        Self { 
            visible : true,
            last_mouse_move : Instant::now(),
            notifications_open: true,
        }
    }

    /// Create the UI using egui.
    fn ui(
        &mut self,
        ctx: &Context,
        ui: &mut egui::Ui,
        app_state: Arc<RwLock<client::ui::state::AppState>>,
        client_state: Arc<client::state::ClientState>,
        media_state: Arc<media::state::MediaState>,
    ) {

        if !self.visible {
            return;
        }

        egui::Panel::top("menubar_container").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("View", |ui| {
                    if ui.button("Notifications").clicked() {
                        self.notifications_open = true;
                        ui.close();
                    }
                })
            });
        });

        egui::Area::new("controls".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -20.0])
        .show(ctx, |ui| {
            egui::Frame::window(ui.style()).show(ui, |ui|{
                ui.horizontal(|ui|{
                
                    if ui.button("Start streaming").clicked(){
                        client_state.set_command(ClientCommand::StartStream);
                    };
                    if ui.button("Stop streaming").clicked(){
                        client_state.set_command(ClientCommand::StopStream);
                    };
                    if ui.button("Disconnect").clicked(){
                        client_state.set_command(ClientCommand::Disconnect);
                    };

                    let streams = media_state.stream_ids();

                    for uid in streams {
                        if ui.button(format!("Watch {uid}")).clicked() {
                            app_state.write().unwrap().selected_stream = Some(uid);
                        }
                    }
                })
            })
        });

        egui::Window::new("Notifications")
        .default_pos([20.0, 20.0])
        .default_size([300.0, 250.0])
        .open(&mut self.notifications_open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui|{
                let app_state = app_state.read().unwrap();
                for notification in &app_state.notifications {
                     ui.label(notification);
                }
            });
        });

    }
}