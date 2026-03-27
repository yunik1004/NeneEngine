//! Fixed-timestep demo — bouncing balls.
//!
//! Physics runs at a fixed 20 Hz (`fixed_update`).
//! Rendering runs at the display's full refresh rate.
//!
//! Set `FIXED_HZ` to 60 to make movement silky-smooth, or keep it at 20 to
//! see that physics ticks fire in bursts while rendering stays fluid.

use std::f32::consts::TAU;

use nene::{
    math::{Mat4, Vec3, Vec4},
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    time::{FixedTime, Time},
    uniform, vertex,
    window::{Config, Window},
};

const FIXED_HZ: f32 = 20.0;
const HALF: f32 = 0.88; // NDC half-extent of the bounding box
const SIDES: usize = 32;

// ── Shader ────────────────────────────────────────────────────────────────────

const SHADER: &str = r#"
@group(0) @binding(0) var<uniform> mvp:   mat4x4<f32>;
@group(1) @binding(0) var<uniform> color: vec4<f32>;

@vertex  fn vs(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return mvp * vec4<f32>(pos, 0.0, 1.0);
}
@fragment fn fs() -> @location(0) vec4<f32> { return color; }
"#;

// ── Vertex / Uniform types ────────────────────────────────────────────────────

#[vertex]
struct Vtx {
    pos: [f32; 2],
}
#[uniform]
struct Mvp {
    mat: Mat4,
}
#[uniform]
struct Color {
    rgba: Vec4,
}

// ── Circle mesh ───────────────────────────────────────────────────────────────

fn circle_mesh() -> (Vec<Vtx>, Vec<u32>) {
    let mut verts = vec![Vtx { pos: [0.0, 0.0] }]; // centre
    for i in 0..=SIDES {
        let a = i as f32 / SIDES as f32 * TAU;
        verts.push(Vtx {
            pos: [a.cos(), a.sin()],
        });
    }
    let mut idx = Vec::new();
    for i in 0..SIDES as u32 {
        idx.extend_from_slice(&[0, i + 1, i + 2]);
    }
    (verts, idx)
}

// ── Physics state ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Ball {
    pos: [f32; 2],
    vel: [f32; 2],
    radius: f32,
    color: Vec4,
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

struct BallGpu {
    mvp_buf: UniformBuffer,
    color_buf: UniformBuffer,
}

struct State {
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ibuf: IndexBuffer,
    idx_count: u32,
    balls: Vec<Ball>,
    gpu: Vec<BallGpu>,
}

fn init(ctx: &mut Context) -> State {
    let (verts, indices) = circle_mesh();
    let idx_count = indices.len() as u32;

    let balls: Vec<Ball> = BALL_SPECS
        .iter()
        .map(|&(px, py, vx, vy, r, c)| Ball {
            pos: [px, py],
            vel: [vx, vy],
            radius: r,
            color: Vec4::new(c[0], c[1], c[2], c[3]),
        })
        .collect();

    let gpu: Vec<BallGpu> = balls
        .iter()
        .map(|_| BallGpu {
            mvp_buf: ctx.create_uniform_buffer(&Mvp {
                mat: Mat4::IDENTITY,
            }),
            color_buf: ctx.create_uniform_buffer(&Color { rgba: Vec4::ONE }),
        })
        .collect();

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, Vtx::layout())
            .with_uniform() // group 0: mvp
            .with_uniform(), // group 1: color
    );

    State {
        pipeline,
        vbuf: ctx.create_vertex_buffer(&verts),
        ibuf: ctx.create_index_buffer(&indices),
        idx_count,
        balls,
        gpu,
    }
}

// ── Callbacks ─────────────────────────────────────────────────────────────────

fn fixed_update(
    state: &mut State,
    _ctx: &mut Context,
    _input: &nene::input::Input,
    ft: &FixedTime,
) {
    let dt = ft.delta;
    for b in &mut state.balls {
        b.pos[0] += b.vel[0] * dt;
        b.pos[1] += b.vel[1] * dt;
        // Bounce off walls
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

fn update(state: &mut State, ctx: &mut Context, _input: &nene::input::Input, _time: &Time) {
    for (ball, gpu) in state.balls.iter().zip(state.gpu.iter()) {
        let t = Mat4::from_translation(Vec3::new(ball.pos[0], ball.pos[1], 0.0));
        let s = Mat4::from_scale(Vec3::new(ball.radius, ball.radius, 1.0));
        ctx.update_uniform_buffer(&gpu.mvp_buf, &Mvp { mat: t * s });
        ctx.update_uniform_buffer(&gpu.color_buf, &Color { rgba: ball.color });
    }
}

fn render(state: &mut State, pass: &mut RenderPass) {
    pass.set_pipeline(&state.pipeline);
    pass.set_vertex_buffer(0, &state.vbuf);
    for gpu in &state.gpu {
        pass.set_uniform(0, &gpu.mvp_buf);
        pass.set_uniform(1, &gpu.color_buf);
        pass.draw_indexed_count(&state.ibuf, state.idx_count);
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    Window::new(Config {
        title: format!("Fixed Update — physics {FIXED_HZ} Hz / render uncapped"),
        ..Config::default()
    })
    .run_with_fixed_update(FIXED_HZ, init, fixed_update, update, render);
}
