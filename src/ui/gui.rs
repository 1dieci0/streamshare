use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use egui::{
    ClippedPrimitive,
    Context,
    TexturesDelta,
    ViewportId,
};

use egui_wgpu::{
    Renderer,
    RendererOptions,
    ScreenDescriptor,
};

use pixels::{
    PixelsContext,
    wgpu,
};

use tokio::sync::mpsc::Sender;

use winit::{
    event_loop::ActiveEventLoop,
    window::Window,
};

use crate::client::command::ClientCommand;

#[derive(Clone)]
pub struct User {
    pub uid: u64,
    pub username: String,
}

#[derive(Clone)]
pub struct Stream {
    pub uid: u64,
    pub username: String,
}

/// Manages all egui rendering state.
pub(crate) struct Framework {
    egui_ctx: Context,
    egui_state: egui_winit::State,

    screen_descriptor: ScreenDescriptor,

    renderer: Renderer,

    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,

    gui: Gui,
}

pub struct Gui {
    visible: bool,
    last_mouse_move: Instant,

    notifications_open: bool,

    command_tx: Sender<ClientCommand>,

    users: HashMap<u64, String>,
    streams: HashMap<u64, String>,

    selected_stream: Option<u64>,
}

impl Framework {
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        width: u32,
        height: u32,
        scale_factor: f32,
        pixels: &pixels::Pixels,
        command_tx: Sender<ClientCommand>,
    ) -> Self {
        let max_texture_size =
            pixels
                .device()
                .limits()
                .max_texture_dimension_2d
                as usize;

        let egui_ctx = Context::default();

        let egui_state =
            egui_winit::State::new(
                egui_ctx.clone(),
                ViewportId::ROOT,
                event_loop,
                Some(scale_factor),
                None,
                Some(max_texture_size),
            );

        let screen_descriptor =
            ScreenDescriptor {
                size_in_pixels: [
                    width,
                    height,
                ],
                pixels_per_point: scale_factor,
            };

        let renderer =
            Renderer::new(
                pixels.device(),
                pixels.render_texture_format(),
                RendererOptions::default(),
            );

        Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            renderer,

            paint_jobs: Vec::new(),
            textures: TexturesDelta::default(),

            gui: Gui::new(command_tx),
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) {
        let _ =
            self.egui_state
                .on_window_event(
                    window,
                    event,
                );
    }

    pub(crate) fn resize(
        &mut self,
        width: u32,
        height: u32,
    ) {
        if width > 0 && height > 0 {
            self.screen_descriptor
                .size_in_pixels = [
                width,
                height,
            ];
        }
    }

    pub(crate) fn scale_factor(
        &mut self,
        scale_factor: f64,
    ) {
        self.screen_descriptor
            .pixels_per_point =
            scale_factor as f32;
    }

    pub(crate) fn set_state(
        &mut self,
        users: HashMap<u64, String>,
        streams: HashMap<u64, String>,
        selected_stream: Option<u64>,
    ) {
        self.gui.users = users;
        self.gui.streams = streams;
        self.gui.selected_stream = selected_stream;
    }

    pub(crate) fn prepare(
        &mut self,
        window: &Window,
    ) {
        let raw_input = self.egui_state.take_egui_input(window);

        let output = self.egui_ctx.run_ui(raw_input, |ui| {
            self.gui.ui(
                &self.egui_ctx,
                ui,
            );
        });

        self.textures.append(output.textures_delta);

        self.egui_state
            .handle_platform_output(
                window,
                output.platform_output,
            );

        self.paint_jobs = self.egui_ctx.tessellate(
            output.shapes,
            self.screen_descriptor.pixels_per_point,
        );
    }

    pub(crate) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) {
        for (id, image_delta)
            in &self.textures.set
        {
            self.renderer.update_texture(
                &context.device,
                &context.queue,
                *id,
                image_delta,
            );
        }

        self.renderer.update_buffers(
            &context.device,
            &context.queue,
            encoder,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        {
            let mut render_pass =
                encoder
                    .begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            label: Some("egui"),

                            color_attachments:
                                &[Some(
                                    wgpu::RenderPassColorAttachment {
                                        view: render_target,
                                        resolve_target: None,

                                        ops:
                                            wgpu::Operations {
                                                load: wgpu::LoadOp::Load,
                                                store: wgpu::StoreOp::Store,
                                            },

                                        depth_slice: None,
                                    },
                                )],

                            depth_stencil_attachment: None,

                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        },
                    )
                    .forget_lifetime();

            self.renderer.render(
                &mut render_pass,
                &self.paint_jobs,
                &self.screen_descriptor,
            );
        }

        let textures =
            std::mem::take(
                &mut self.textures
            );

        for id in &textures.free {
            self.renderer
                .free_texture(id);
        }
    }

    pub fn mouse_moved(&mut self) {
        self.gui.visible = true;
        self.gui.last_mouse_move =
            Instant::now();
    }

    pub fn update(&mut self) {
        if self.gui.last_mouse_move.elapsed()
            > Duration::from_secs(3)
        {
            self.gui.visible = false;
        }
    }

    pub fn needs_repaint(&self) -> bool {
        self.egui_ctx
            .has_requested_repaint()
    }
}

impl Gui {
    fn new(
        command_tx: Sender<ClientCommand>,
    ) -> Self {
        Self {
            visible: true,
            last_mouse_move: Instant::now(),

            notifications_open: true,

            command_tx,

            users: HashMap::new(),
            streams: HashMap::new(),

            selected_stream: None,
        }
    }

    fn send_command(
        &self,
        command: ClientCommand,
    ) {
        let tx =
            self.command_tx.clone();

        // UI must never block waiting for the
        // network layer.
        tokio::spawn(async move {
            if let Err(e) =
                tx.send(command).await
            {
                eprintln!(
                    "failed to send command: {e}"
                );
            }
        });
    }

    fn ui(
        &mut self,
        ctx: &Context,
        ui: &mut egui::Ui,
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
                });
            });
        });

        egui::Area::new(
            "controls".into(),
        )
        .anchor(
            egui::Align2::CENTER_BOTTOM,
            [0.0, -20.0],
        )
        .show(ctx, |ui| {
            egui::Frame::window(
                ui.style(),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            "Start streaming",
                        )
                        .clicked()
                    {
                        self.send_command(
                            ClientCommand::StartStream,
                        );
                    }

                    if ui
                        .button(
                            "Stop streaming",
                        )
                        .clicked()
                    {
                        self.send_command(
                            ClientCommand::StopStream,
                        );
                    }

                    for (&uid, username)
                        in &self.streams
                    {
                        let selected =
                            self.selected_stream
                                == Some(uid);

                        let label =
                            if selected {
                                format!(
                                    "Watching {}",
                                    username
                                )
                            } else {
                                format!(
                                    "Watch {}",
                                    username
                                )
                            };

                        if ui
                            .button(label)
                            .clicked()
                        {
                            self.selected_stream =
                                Some(uid);

                            self.send_command(
                                ClientCommand::WatchStream {
                                    uid,
                                },
                            );
                        }
                    }
                });
            });
        });

        egui::Window::new(
            "Notifications",
        )
        .default_pos([20.0, 20.0])
        .default_size([300.0, 250.0])
        .open(
            &mut self.notifications_open,
        )
        .show(ctx, |ui| {
            ui.label(
                "Connected users:",
            );

            for (uid, username)
                in &self.users
            {
                ui.label(
                    format!(
                        "{uid}: {username}"
                    ),
                );
            }

            ui.separator();

            ui.label(
                "Active streams:",
            );

            for (uid, username)
                in &self.streams
            {
                ui.label(
                    format!(
                        "{uid}: {username}"
                    ),
                );
            }
        });
    }
}