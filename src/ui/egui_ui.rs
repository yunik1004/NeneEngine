use std::sync::Arc;

use winit::event::WindowEvent;
use winit::window::Window;

use crate::renderer::{Context, RenderPass};

/// egui UI context — wraps egui + egui-wgpu + egui-winit.
///
/// # Usage
///
/// ```no_run
/// use nene::app::{App, Config, WindowId, WindowEvent, run};
/// use nene::renderer::{Context, RenderPass};
/// use nene::ui::EguiUi;
///
/// struct MyApp { egui: Option<EguiUi> }
///
/// impl App for MyApp {
///     fn new() -> Self { MyApp { egui: None } }
///
///     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
///         self.egui = Some(EguiUi::new(ctx));
///     }
///
///     fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
///         if let Some(e) = &mut self.egui { e.handle_event(event); }
///     }
///
///     fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &nene::input::Input) {
///         let Some(egui) = &mut self.egui else { return };
///         let ui = egui.begin_frame();
///         egui::Window::new("Hello").show(&ui, |ui| { ui.label("World"); });
///         egui.end_frame(ctx);
///     }
///
///     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
///         if let Some(e) = &self.egui { e.render(pass); }
///     }
/// }
/// ```
pub struct EguiUi {
    ctx: egui::Context,
    renderer: egui_wgpu::Renderer,
    state: egui_winit::State,
    paint_jobs: Vec<egui::ClippedPrimitive>,
    screen_desc: egui_wgpu::ScreenDescriptor,
    free_textures: Vec<egui::TextureId>,
    window: Arc<Window>,
}

impl EguiUi {
    /// Create from a nene [`Context`] (call inside [`App::window_ready`]).
    pub fn new(ctx: &Context) -> Self {
        let egui_ctx = egui::Context::default();
        let window = ctx.window().clone();

        let state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            Some(2048),
        );

        let renderer = egui_wgpu::Renderer::new(
            ctx.device(),
            ctx.surface_config().format,
            egui_wgpu::RendererOptions {
                // Match the depth attachment used by the engine's render pass.
                depth_stencil_format: Some(wgpu::TextureFormat::Depth32Float),
                ..egui_wgpu::RendererOptions::default()
            },
        );

        let sc = ctx.surface_config();
        Self {
            ctx: egui_ctx,
            renderer,
            state,
            paint_jobs: Vec::new(),
            screen_desc: egui_wgpu::ScreenDescriptor {
                size_in_pixels: [sc.width, sc.height],
                pixels_per_point: window.scale_factor() as f32,
            },
            free_textures: Vec::new(),
            window,
        }
    }

    /// Forward a raw winit event (call from [`App::on_window_event`]).
    pub fn handle_event(&mut self, event: &WindowEvent) {
        let _ = self.state.on_window_event(&self.window, event);
    }

    /// Begin a new UI frame. Use the returned [`egui::Context`] to build widgets.
    ///
    /// Must be paired with [`end_frame`](Self::end_frame).
    pub fn begin_frame(&mut self) -> egui::Context {
        let raw_input = self.state.take_egui_input(&self.window);
        self.ctx.begin_pass(raw_input);
        self.ctx.clone()
    }

    /// Finish the frame, tessellate shapes, and upload buffers to the GPU.
    ///
    /// Call in [`App::prepare`] after all widget calls.
    pub fn end_frame(&mut self, ctx: &mut Context) {
        let output = self.ctx.end_pass();
        self.state
            .handle_platform_output(&self.window, output.platform_output);

        let sc = ctx.surface_config();
        self.screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [sc.width, sc.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        // Free textures uploaded in a previous frame.
        for id in self.free_textures.drain(..) {
            self.renderer.free_texture(&id);
        }

        let ppp = self.screen_desc.pixels_per_point;
        self.paint_jobs = self.ctx.tessellate(output.shapes, ppp);

        for (id, delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(ctx.device(), ctx.queue(), *id, delta);
        }
        self.free_textures = output.textures_delta.free;

        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let extra = self.renderer.update_buffers(
            ctx.device(),
            ctx.queue(),
            &mut encoder,
            &self.paint_jobs,
            &self.screen_desc,
        );
        ctx.queue()
            .submit([encoder.finish()].into_iter().chain(extra));
    }

    /// Render into the current pass (call from [`App::render`]).
    pub fn render(&self, pass: &mut RenderPass) {
        // SAFETY: egui_wgpu::Renderer::render only issues draw commands within
        // the scope of the render pass; it does not store the reference.
        let rpass: &mut wgpu::RenderPass<'static> = unsafe { std::mem::transmute(&mut pass.inner) };
        self.renderer
            .render(rpass, &self.paint_jobs, &self.screen_desc);
    }
}
