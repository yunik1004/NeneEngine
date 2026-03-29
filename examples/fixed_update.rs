//! Fixed-timestep + profiler demo — bouncing balls.
//!
//! Physics runs at a fixed 60 Hz (`fixed_update`).
//! Rendering runs at the display's full refresh rate.
//! A profiler panel shows frame time, FPS, and per-scope timings.
//!
//! Set `FIXED_HZ` to 20 to see that physics ticks fire in bursts while
//! rendering stays fluid.

use nene::{
    app::{App, Config, FixedApp, WindowId, run_fixed},
    debug::Profiler,
    input::Input,
    math::Mat4,
    mesh::{Vertex, circle_segments},
    renderer::{Context, GpuMesh, Material, MaterialBuilder, RenderPass},
    time::{FixedTime, Time},
    ui::Ui,
};

const FIXED_HZ: f32 = 60.0;
const HALF: f32 = 0.88;
const SIDES: u32 = 32;

// ── Physics state ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Ball {
    pos: [f32; 2],
    vel: [f32; 2],
    radius: f32,
    color: [f32; 4],
}

const BALL_SPECS: [(f32, f32, f32, f32, f32, [f32; 4]); 6] = [
    (0.0, 0.3, 0.38, 0.55, 0.07, [1.0, 0.3, 0.1, 1.0]),
    (-0.4, -0.2, -0.42, 0.31, 0.06, [0.2, 0.8, 0.3, 1.0]),
    (0.5, 0.1, 0.25, -0.60, 0.05, [0.2, 0.5, 1.0, 1.0]),
    (-0.6, 0.5, -0.30, -0.40, 0.08, [1.0, 0.8, 0.1, 1.0]),
    (0.3, -0.6, 0.50, 0.35, 0.04, [0.9, 0.3, 0.9, 1.0]),
    (0.0, 0.0, -0.20, 0.70, 0.09, [0.1, 0.9, 0.9, 1.0]),
];

// ── App state ─────────────────────────────────────────────────────────────────

struct FixedUpdateDemo {
    balls: Vec<Ball>,
    profiler: Profiler,
    mat: Option<Material>,
    mesh: Option<GpuMesh>,
    ui: Option<Ui>,
}

impl App for FixedUpdateDemo {
    fn new() -> Self {
        let balls = BALL_SPECS
            .iter()
            .map(|&(px, py, vx, vy, r, c)| Ball {
                pos: [px, py],
                vel: [vx, vy],
                radius: r,
                color: c,
            })
            .collect();

        FixedUpdateDemo {
            balls,
            profiler: Profiler::new(),
            mat: None,
            mesh: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.mat = Some(MaterialBuilder::new().vertex_color().build(ctx));
        self.mesh = Some(GpuMesh::new(ctx, &[], &[]));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, _input: &Input, _time: &Time) {
        self.profiler.begin_frame();
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, input: &Input) {
        let _s = self.profiler.scope("update");

        let mut verts: Vec<Vertex> = Vec::new();
        for ball in &self.balls {
            verts.extend_from_slice(&circle_segments(
                ball.pos[0],
                ball.pos[1],
                ball.radius,
                ball.color,
                SIDES,
            ));
        }
        if let Some(mesh) = &mut self.mesh {
            mesh.update(ctx, &verts);
        }
        if let Some(mat) = &mut self.mat {
            mat.uniform.view_proj = Mat4::IDENTITY;
            mat.flush(ctx);
        }
        drop(_s);

        let cfg = ctx.surface_config();
        if let Some(ui) = &mut self.ui {
            ui.begin_frame(input, cfg.width as f32, cfg.height as f32);
            self.profiler.draw_overlay(ui, 16.0, 16.0);
            ui.end_frame(ctx);
        }
        self.profiler.end_frame();
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let (Some(mat), Some(mesh)) = (&self.mat, &self.mesh) {
            mat.render(pass, mesh);
        }
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Fixed Update",
            ..Config::default()
        }]
    }
}

impl FixedApp for FixedUpdateDemo {
    fn fixed_update(&mut self, _input: &Input, ft: &FixedTime) {
        let _s = self.profiler.scope("fixed_update");
        let dt = ft.delta;
        for b in &mut self.balls {
            b.pos[0] += b.vel[0] * dt;
            b.pos[1] += b.vel[1] * dt;
            if b.pos[0] + b.radius > HALF {
                b.pos[0] = HALF - b.radius;
                b.vel[0] = -b.vel[0].abs();
            }
            if b.pos[0] - b.radius < -HALF {
                b.pos[0] = -HALF + b.radius;
                b.vel[0] = b.vel[0].abs();
            }
            if b.pos[1] + b.radius > HALF {
                b.pos[1] = HALF - b.radius;
                b.vel[1] = -b.vel[1].abs();
            }
            if b.pos[1] - b.radius < -HALF {
                b.pos[1] = -HALF + b.radius;
                b.vel[1] = b.vel[1].abs();
            }
        }
    }
}

fn main() {
    run_fixed::<FixedUpdateDemo>(FIXED_HZ);
}
