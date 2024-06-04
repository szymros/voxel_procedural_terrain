use egui::*;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

// let surface_texture = state.surface
// .get_current_texture()
// .expect("Failed to acquire next swap chain texture");
//
// let surface_view = surface_texture
// .texture
// .create_view(&wgpu::TextureViewDescriptor::default());
//
// let mut encoder =
// state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
// label: None,
// });
//
// let screen_descriptor = ScreenDescriptor {
// size_in_pixels: [800, 600],
// pixels_per_point: window.scale_factor() as f32 * scale_factor,
// };

// egui_renderer.draw(
// &state.device,
// &state.queue,
// &mut encoder,
// &window,
// &surface_view,
// screen_descriptor,
// |ctx| {
// egui::Window::new("winit + egui + wgpu says hello!")
// .resizable(true)
// .vscroll(true)
// .default_open(false)
// .show(&ctx, |ui| {
// ui.label("Label!");
//
// if ui.button("Button!").clicked() {
// println!("boom!")
// }
//
// ui.separator();
// ui.horizontal(|ui| {
// ui.label(format!(
// "Pixels per point: {}",
// ctx.pixels_per_point()
// ));
// if ui.button("-").clicked() {
// scale_factor = (scale_factor - 0.1).max(0.3);
// }
// if ui.button("+").clicked() {
// scale_factor = (scale_factor + 0.1).min(3.0);
// }
// });
// });
// },
// );
//
// state.queue.submit(Some(encoder.finish()));
// surface_texture.present();
//

pub struct GuiRenderer {
    state: State,
    renderer: Renderer,
}

impl GuiRenderer {
    pub fn context(&self) -> &Context {
        return self.state.egui_ctx();
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> GuiRenderer {
        let egui_context = Context::default();

        let mut egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
        );

        GuiRenderer {
            state: egui_state,
            renderer: egui_renderer,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) {
        self.state.on_window_event(window, &event);
    }

    pub fn ppp(&mut self, v: f32) {
        self.state.egui_ctx().set_pixels_per_point(v);
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        run_ui: impl FnOnce(&Context),
    ) {
        self.state
            .egui_ctx()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        let raw_input = self.state.take_egui_input(&window);
        let full_output = self.state.egui_ctx().run(raw_input, |ui| {
            run_ui(&self.state.egui_ctx());
        });

        self.state
            .handle_platform_output(&window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(&device, &queue, *id, &image_delta);
        }
        self.renderer
            .update_buffers(&device, &queue, encoder, &tris, &screen_descriptor);
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        drop(rpass);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}