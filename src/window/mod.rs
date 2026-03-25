use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window as WinitWindow, WindowId},
};

use crate::{input::Input, renderer::Context, time::Time};

pub struct Config {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "Nene".to_string(),
            width: 1280,
            height: 720,
        }
    }
}

pub struct Window {
    config: Config,
}

impl Window {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn run(self) {
        self.run_with(|_| (), |_, _| {});
    }

    pub fn run_with<S: 'static>(
        self,
        init: impl FnOnce(&mut Context) -> S + 'static,
        render: impl FnMut(&mut S, &mut crate::renderer::RenderPass<'_>) + 'static,
    ) {
        self.run_with_update(init, |_, _, _, _| {}, |_, _| {}, render);
    }

    /// Like [`run_with`](Self::run_with) but with an `update` callback that runs before
    /// each frame's render pass.
    ///
    /// The `update` closure receives `(&mut S, &mut Context, &Input, &Time)`.
    pub fn run_with_update<S: 'static>(
        self,
        init: impl FnOnce(&mut Context) -> S + 'static,
        update: impl FnMut(&mut S, &mut Context, &Input, &Time) + 'static,
        pre_render: impl FnMut(&mut S, &mut Context) + 'static,
        render: impl FnMut(&mut S, &mut crate::renderer::RenderPass<'_>) + 'static,
    ) {
        let now = Instant::now();
        let mut runner = WindowRunner {
            config: self.config,
            handle: None,
            renderer: None,
            input: Input::new(),
            time: Time {
                delta: 0.0,
                elapsed: 0.0,
                frame: 0,
            },
            last_frame: now,
            start: now,
            init: Some(Box::new(init)),
            update: Box::new(update),
            pre_render: Box::new(pre_render),
            render: Box::new(render),
            state: None,
        };
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut runner).expect("Event loop error");
    }
}

type InitFn<S> = Box<dyn FnOnce(&mut Context) -> S>;
type UpdateFn<S> = Box<dyn FnMut(&mut S, &mut Context, &Input, &Time)>;
type PreRenderFn<S> = Box<dyn FnMut(&mut S, &mut Context)>;
type RenderFn<S> = Box<dyn FnMut(&mut S, &mut crate::renderer::RenderPass<'_>)>;

struct WindowRunner<S> {
    config: Config,
    handle: Option<Arc<WinitWindow>>,
    renderer: Option<Context>,
    input: Input,
    time: Time,
    last_frame: Instant,
    start: Instant,
    init: Option<InitFn<S>>,
    update: UpdateFn<S>,
    pre_render: PreRenderFn<S>,
    render: RenderFn<S>,
    state: Option<S>,
}

impl<S> ApplicationHandler for WindowRunner<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    WinitWindow::default_attributes()
                        .with_title(&self.config.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.config.width,
                            self.config.height,
                        )),
                )
                .expect("Failed to create window"),
        );

        let mut ctx = Context::new(Arc::clone(&window));

        if let Some(init) = self.init.take() {
            self.state = Some(init(&mut ctx));
        }

        // Reset clock so the first frame delta isn't inflated by init time.
        self.last_frame = Instant::now();
        self.start = self.last_frame;

        self.renderer = Some(ctx);
        self.handle = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.on_key(event.physical_key, event.state);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.on_mouse_button(button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input
                    .on_cursor_moved(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.on_scroll(delta);
            }
            WindowEvent::RedrawRequested => {
                // Update timing.
                let now = Instant::now();
                let raw_delta = now.duration_since(self.last_frame);
                // Clamp delta to 250 ms to avoid spiral-of-death on focus loss.
                let delta = raw_delta.min(Duration::from_millis(250));
                self.last_frame = now;
                self.time = Time {
                    delta: delta.as_secs_f32(),
                    elapsed: now.duration_since(self.start).as_secs_f64(),
                    frame: self.time.frame + 1,
                };

                self.input.process_gilrs();

                if let (Some(ctx), Some(state)) = (&mut self.renderer, &mut self.state) {
                    (self.update)(state, ctx, &self.input, &self.time);
                    (self.pre_render)(state, ctx);
                    let render = &mut self.render;
                    ctx.render_with(|pass| render(state, pass));
                }

                self.input.begin_frame();
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            self.input.on_mouse_motion(dx, dy);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(handle) = &self.handle {
            handle.request_redraw();
        }
    }
}
