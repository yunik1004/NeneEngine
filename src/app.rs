//! Trait-based entry point for nene apps.
//!
//! The [`App`] trait is **window-independent**: `App::new` runs before any
//! window exists. Windows are declared via `App::windows()` and opened by the
//! runner. Each open window calls `App::window_ready`, `App::prepare`, and
//! `App::render` independently, so multiple windows Just Work.
//!
//! # Lifecycle
//!
//! ```text
//! App::new()               — pure init, no GPU, no window
//! for each window:
//!   App::window_ready()    — GPU resource creation for that window
//! loop:
//!   App::update()          — game logic, once per frame (no GPU access)
//!   for each window:
//!     App::prepare()       — GPU buffer uploads for that window
//!     App::render()        — draw calls for that window
//! ```
//!
//! # Single-window example
//!
//! ```no_run
//! use nene::app::{App, WindowId, run};
//! use nene::input::Input;
//! use nene::math::{Mat4, Vec4};
//! use nene::renderer::{Context, Material, MaterialBuilder, Mesh, RenderPass};
//! use nene::time::Time;
//! use nene::window::Config;
//! use nene::mesh::MeshVertex;
//!
//! struct MyGame {
//!     mat:  Option<Material>,
//!     mesh: Option<Mesh>,
//! }
//!
//! impl App for MyGame {
//!     fn new() -> Self { MyGame { mat: None, mesh: None } }
//!
//!     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
//!         self.mat  = Some(MaterialBuilder::new().ambient().build(ctx));
//!         self.mesh = Some(Mesh::new(ctx, &[], &[]));
//!     }
//!
//!     fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
//!         let Some(mat) = &mut self.mat else { return };
//!         mat.uniform.view_proj = Mat4::IDENTITY;
//!         mat.flush(ctx);
//!     }
//!
//!     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
//!         let (Some(mat), Some(mesh)) = (&self.mat, &self.mesh) else { return };
//!         mat.render(pass, mesh);
//!     }
//!
//!     fn windows() -> Vec<Config> {
//!         vec![Config { title: "My Game", ..Config::default() }]
//!     }
//! }
//!
//! fn main() { run::<MyGame>(); }
//! ```
//!
//! # Multi-window example
//!
//! ```no_run
//! # use nene::app::{App, WindowId, run};
//! # use nene::renderer::{Context, RenderPass};
//! # use nene::window::Config;
//! struct MultiWin { main: Option<WindowId>, hud: Option<WindowId> }
//!
//! impl App for MultiWin {
//!     fn new() -> Self { MultiWin { main: None, hud: None } }
//!
//!     fn window_ready(&mut self, id: WindowId, _ctx: &mut Context) {
//!         // First window opened → main; second → hud.
//!         if self.main.is_none() { self.main = Some(id); }
//!         else                   { self.hud  = Some(id); }
//!     }
//!
//!     fn render(&mut self, id: WindowId, pass: &mut RenderPass) {
//!         if Some(id) == self.main { /* scene */   }
//!         else                     { /* hud only */ }
//!     }
//!
//!     fn windows() -> Vec<Config> {
//!         vec![
//!             Config { title: "Scene", width: 1280, height: 720, ..Config::default() },
//!             Config { title: "HUD",   width:  400, height: 300, ..Config::default() },
//!         ]
//!     }
//! }
//! # fn main() { run::<MultiWin>(); }
//! ```

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window as WinitWindow,
};

use crate::{
    input::Input,
    renderer::{Context, RenderPass},
    time::{FixedTime, Time},
    window::{Config, MAX_FIXED_STEPS},
};

pub use winit::window::WindowId;

// ── App trait ─────────────────────────────────────────────────────────────────

/// Core app trait. Implement on your state struct, then call [`run::<MyApp>()`].
///
/// All methods have default no-op implementations except [`new`](Self::new).
pub trait App: Sized + 'static {
    /// Pure initialization — no GPU context, no windows yet.
    ///
    /// Store game-logic state here; defer GPU resource creation to
    /// [`window_ready`](Self::window_ready).
    fn new() -> Self;

    /// Called once per window immediately after it opens.
    ///
    /// Create pipelines, vertex buffers, and other per-window GPU resources.
    fn window_ready(&mut self, _id: WindowId, _ctx: &mut Context) {}

    /// Called once per frame before any window is rendered.
    ///
    /// Run game systems (ECS, physics, AI) and process input here.
    /// No GPU context is available — that comes in [`prepare`](Self::prepare).
    fn update(&mut self, _input: &Input, _time: &Time) {}

    /// Called once per window per frame after [`update`](Self::update).
    ///
    /// Upload uniform/vertex/instance buffer data for this window.
    /// Also the right place for UI `end_frame` and similar GPU-touching work.
    fn prepare(&mut self, _id: WindowId, _ctx: &mut Context, _input: &Input) {}

    /// Called once per window per frame to issue draw calls.
    fn render(&mut self, _id: WindowId, _pass: &mut RenderPass) {}

    /// Called when a window is closed by the user.
    ///
    /// The window is removed automatically; this callback lets you react
    /// (e.g. shut down if the main window closes).
    fn window_closed(&mut self, _id: WindowId) {}

    /// Windows to open at startup. Default: one 1280×720 window titled "Nene".
    fn windows() -> Vec<Config> {
        vec![Config::default()]
    }
}

// ── FixedApp trait ────────────────────────────────────────────────────────────

/// Extension of [`App`] for apps that need a fixed-timestep logic tick.
///
/// Use [`run_fixed::<MyApp>(hz)`] to enter the loop.
///
/// `fixed_update` runs 0–[`MAX_FIXED_STEPS`] times per frame at a constant
/// `delta = 1 / hz` before the variable-rate [`update`](App::update) and the
/// per-window [`prepare`](App::prepare) + [`render`](App::render).
pub trait FixedApp: App {
    fn fixed_update(&mut self, input: &Input, time: &FixedTime);
}

// ── Internal window wrapper ───────────────────────────────────────────────────

struct ManagedWindow {
    handle: Arc<WinitWindow>,
    ctx: Context,
}

// ── AppRunner ─────────────────────────────────────────────────────────────────

struct AppRunner<A: App> {
    app: Option<A>,
    pending_configs: Vec<Config>,
    windows: HashMap<WindowId, ManagedWindow>,
    input: Input,
    time: Time,
    last_frame: Instant,
    start: Instant,
}

impl<A: App> AppRunner<A> {
    fn new(configs: Vec<Config>) -> Self {
        let now = Instant::now();
        Self {
            app: None,
            pending_configs: configs,
            windows: HashMap::new(),
            input: Input::new(),
            time: Time {
                delta: 0.0,
                elapsed: 0.0,
                frame: 0,
            },
            last_frame: now,
            start: now,
        }
    }
}

impl<A: App> ApplicationHandler for AppRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let app = self.app.get_or_insert_with(A::new);

        for config in self.pending_configs.drain(..) {
            let handle = Arc::new(
                event_loop
                    .create_window(
                        WinitWindow::default_attributes()
                            .with_title(config.title)
                            .with_inner_size(winit::dpi::LogicalSize::new(
                                config.width,
                                config.height,
                            )),
                    )
                    .expect("failed to create window"),
            );
            let mut ctx = Context::new(Arc::clone(&handle));
            let id = handle.id();
            app.window_ready(id, &mut ctx);
            self.windows.insert(id, ManagedWindow { handle, ctx });
        }

        let now = Instant::now();
        self.last_frame = now;
        self.start = now;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(app) = &mut self.app {
                    app.window_closed(id);
                }
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(win) = self.windows.get_mut(&id) {
                    win.ctx.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.on_key(event.physical_key, event.state);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.on_mouse_button(button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .windows
                    .get(&id)
                    .map_or(1.0, |w| w.handle.scale_factor());
                self.input
                    .on_cursor_moved((position.x / scale) as f32, (position.y / scale) as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.on_scroll(delta);
            }
            WindowEvent::RedrawRequested => {
                // Rust field-disjoint borrows: self.app, self.windows, self.input are separate.
                let (app_opt, win_opt) = (&mut self.app, self.windows.get_mut(&id));
                if let (Some(app), Some(win)) = (app_opt, win_opt) {
                    app.prepare(id, &mut win.ctx, &self.input);
                    win.ctx.render_with(|pass| app.render(id, pass));
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            self.input.on_mouse_motion(dx, dy);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.windows.is_empty() {
            return;
        }

        let now = Instant::now();
        let dt = now
            .duration_since(self.last_frame)
            .min(Duration::from_millis(250))
            .as_secs_f32();
        self.last_frame = now;
        self.time = Time {
            delta: dt,
            elapsed: now.duration_since(self.start).as_secs_f64(),
            frame: self.time.frame + 1,
        };
        self.input.process_gilrs();

        if let Some(app) = &mut self.app {
            app.update(&self.input, &self.time);
        }

        // begin_frame AFTER update so keys_pressed isn't cleared before update sees it.
        // (RedrawRequested and KeyboardInput can arrive in the same event batch before
        // about_to_wait fires, so clearing in RedrawRequested loses same-batch key events.)
        self.input.begin_frame();

        for win in self.windows.values() {
            win.handle.request_redraw();
        }
    }
}

// ── FixedAppRunner ────────────────────────────────────────────────────────────

struct FixedAppRunner<A: FixedApp> {
    inner: AppRunner<A>,
    fixed_step: f32,
    accumulator: f32,
    tick: u64,
}

impl<A: FixedApp> FixedAppRunner<A> {
    fn new(hz: f32, configs: Vec<Config>) -> Self {
        Self {
            inner: AppRunner::new(configs),
            fixed_step: 1.0 / hz,
            accumulator: 0.0,
            tick: 0,
        }
    }
}

impl<A: FixedApp> ApplicationHandler for FixedAppRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.resumed(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.inner.window_event(event_loop, id, event);
    }

    fn device_event(&mut self, el: &ActiveEventLoop, did: DeviceId, event: DeviceEvent) {
        self.inner.device_event(el, did, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.inner.windows.is_empty() {
            return;
        }

        let now = Instant::now();
        let frame_dt = now
            .duration_since(self.inner.last_frame)
            .min(Duration::from_millis(250))
            .as_secs_f32();
        self.inner.last_frame = now;
        self.inner.time = Time {
            delta: frame_dt,
            elapsed: now.duration_since(self.inner.start).as_secs_f64(),
            frame: self.inner.time.frame + 1,
        };
        self.inner.input.process_gilrs();

        // Fixed-timestep ticks
        self.accumulator += frame_dt;
        let max_acc = self.fixed_step * MAX_FIXED_STEPS as f32;
        if self.accumulator > max_acc {
            self.accumulator = max_acc;
        }
        let mut step = 0u32;
        while self.accumulator >= self.fixed_step {
            if let Some(app) = &mut self.inner.app {
                app.fixed_update(
                    &self.inner.input,
                    &FixedTime {
                        delta: self.fixed_step,
                        step,
                        tick: self.tick,
                    },
                );
            }
            self.accumulator -= self.fixed_step;
            self.tick += 1;
            step += 1;
        }

        // Variable-rate update
        if let Some(app) = &mut self.inner.app {
            app.update(&self.inner.input, &self.inner.time);
        }

        self.inner.input.begin_frame();

        for win in self.inner.windows.values() {
            win.handle.request_redraw();
        }

        let _ = event_loop;
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Run an [`App`], opening the windows declared by [`App::windows`] and
/// entering the event loop. Blocks until all windows are closed.
pub fn run<A: App>() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut AppRunner::<A>::new(A::windows()))
        .expect("event loop error");
}

/// Run a [`FixedApp`] with a fixed-timestep logic tick at `hz` Hz, plus a
/// variable-rate [`update`](App::update) and per-window prepare + render.
///
/// Blocks until all windows are closed.
pub fn run_fixed<A: FixedApp>(hz: f32) {
    assert!(hz > 0.0, "hz must be positive");
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut FixedAppRunner::<A>::new(hz, A::windows()))
        .expect("event loop error");
}
