//! Debug draw demo — all primitive types in one scene.
//!
//! Shows how to use [`DebugDraw`] to visualise geometry at runtime.
//!
//! Primitives on screen
//! ────────────────────
//!   • World-origin axes (X=red, Y=green, Z=blue)
//!   • Orbit of the camera shown as a circle
//!   • A grid of spheres that pulse in size over time
//!   • AABB around the sphere grid
//!   • Light-direction ray from the origin
//!   • Cylinders connecting adjacent spheres

use nene::{
    debug::{DebugDraw, color},
    math::{Mat4, Vec3},
    renderer::{Context, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer},
    uniform,
    window::{Config, Window},
};

// ── Minimal background shader (solid dark grey) ───────────────────────────────

const BG_SHADER: &str = r#"
struct U { view_proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> u: U;

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    // Fullscreen triangle
    let x = f32((vi & 1u) * 4u) - 1.0;
    let y = f32((vi & 2u) * 2u) - 1.0;
    return vec4<f32>(x, y, 0.9999, 1.0);
}

@fragment fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.08, 0.08, 0.10, 1.0);
}
"#;

#[uniform]
struct SceneUniform {
    view_proj: Mat4,
}

// ── Grid layout ───────────────────────────────────────────────────────────────

const COLS: i32 = 5;
const ROWS: i32 = 5;
const SPACING: f32 = 2.5;

fn grid_center(col: i32, row: i32) -> Vec3 {
    Vec3::new(
        (col - COLS / 2) as f32 * SPACING,
        0.0,
        (row - ROWS / 2) as f32 * SPACING,
    )
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    bg_pipeline: Pipeline,
    scene_buf: UniformBuffer,
    debug: DebugDraw,
}

fn init(ctx: &mut Context) -> State {
    let bg_pipeline = ctx.create_pipeline(PipelineDescriptor::fullscreen_pass(BG_SHADER));
    let scene_buf = ctx.create_uniform_buffer(&SceneUniform {
        view_proj: Mat4::IDENTITY,
    });
    let debug = DebugDraw::new(ctx);
    State {
        bg_pipeline,
        scene_buf,
        debug,
    }
}

fn main() {
    Window::new(Config {
        title: "Debug Draw — spheres, AABBs, axes, rays, cylinders".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, _input, time| {
            let t = time.elapsed as f32;
            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;

            // ── Camera ─────────────────────────────────────────────────────────
            let cam_r = 18.0_f32;
            let cam_pos = Vec3::new(
                cam_r * (t * 0.18).cos(),
                cam_r * 0.4,
                cam_r * (t * 0.18).sin(),
            );
            let view_proj = Mat4::perspective_rh(50_f32.to_radians(), aspect, 0.1, 100.0)
                * Mat4::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);

            ctx.update_uniform_buffer(&state.scene_buf, &SceneUniform { view_proj });

            // ── Populate debug draw ────────────────────────────────────────────

            // World axes at origin.
            state.debug.axes(Vec3::ZERO, 3.0);

            // Camera orbit circle.
            state
                .debug
                .circle(Vec3::new(0.0, cam_pos.y, 0.0), Vec3::Y, cam_r, color::GRAY);

            // Light direction ray.
            let light_dir = Vec3::new(-0.6, -1.0, -0.4).normalize();
            state
                .debug
                .ray(Vec3::new(0.0, 5.0, 0.0), light_dir, 4.0, color::YELLOW);

            // Grid of pulsing spheres.
            let grid_min = Vec3::new(
                -(COLS / 2) as f32 * SPACING - 1.0,
                -1.5,
                -(ROWS / 2) as f32 * SPACING - 1.0,
            );
            let grid_max = Vec3::new(
                (COLS / 2) as f32 * SPACING + 1.0,
                1.5,
                (ROWS / 2) as f32 * SPACING + 1.0,
            );
            state.debug.aabb(grid_min, grid_max, color::WHITE);

            let mut prev_centers: Vec<Vec3> = Vec::new();
            for row in 0..ROWS {
                for col in 0..COLS {
                    let base = grid_center(col, row);
                    let phase = (col + row * COLS) as f32 * 0.4;
                    let r = 0.4 + 0.2 * (t * 1.5 + phase).sin();
                    let y = 0.5 * (t * 0.8 + phase).sin();
                    let center = Vec3::new(base.x, y, base.z);

                    // Colour cycles through hue via a simple RGB approximation.
                    let hue = (phase / (COLS * ROWS) as f32 + t * 0.1).fract();
                    let sphere_color = hsv(hue, 0.8, 0.9);

                    state.debug.sphere(center, r, sphere_color);

                    // Cylinder to the previous sphere in the row.
                    if col > 0 {
                        let prev = prev_centers[prev_centers.len() - 1];
                        state.debug.cylinder(prev, center, 0.05, color::GRAY);
                    }
                    prev_centers.push(center);
                }
            }

            state.debug.flush(ctx, view_proj);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            // Dark background.
            pass.set_pipeline(&state.bg_pipeline);
            pass.draw(0..3);

            // All debug primitives in one draw call.
            state.debug.draw(pass);
        },
    );
}

// ── Minimal HSV → RGB ─────────────────────────────────────────────────────────

fn hsv(h: f32, s: f32, v: f32) -> Vec3 {
    let i = (h * 6.0) as u32;
    let f = h * 6.0 - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Vec3::new(r, g, b)
}
