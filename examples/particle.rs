//! Particle system demo.
//!
//! Two emitters:
//!   • Continuous fire column rising from the ground (additive blending).
//!   • Spark burst on Space press.
//!
//! Controls
//! ─────────
//!   Space — trigger a spark burst at the fire position

use nene::{
    input::Key,
    math::{Mat4, Vec3},
    particle::{EmitterConfig, ParticleSystem},
    renderer::{Context, RenderPass},
    window::{Config, Window},
};

// ── Camera helpers ─────────────────────────────────────────────────────────────

const W: f32 = 960.0;
const H: f32 = 600.0;

fn camera_view_proj(time: f32) -> (Mat4, Vec3, Vec3) {
    // Orbit camera
    let dist = 12.0_f32;
    let eye_x = dist * (time * 0.3).cos();
    let eye_z = dist * (time * 0.3).sin();
    let eye = Vec3::new(eye_x, 5.0, eye_z);
    let target = Vec3::new(0.0, 2.0, 0.0);
    let up = Vec3::Y;

    let view = Mat4::look_at_rh(eye, target, up);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, W / H, 0.1, 200.0);
    let view_proj = proj * view;

    // Extract billboard axes from view matrix rows (inverse of transpose = view itself)
    let cam_right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
    let cam_up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);

    (view_proj, cam_right, cam_up)
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    fire: ParticleSystem,
    sparks: ParticleSystem,
    elapsed: f32,
}

fn init(ctx: &mut Context) -> State {
    let fire = ParticleSystem::new(ctx, EmitterConfig::fire());
    let sparks = ParticleSystem::new(ctx, EmitterConfig::sparks());
    State {
        fire,
        sparks,
        elapsed: 0.0,
    }
}

fn main() {
    Window::new(Config {
        title: "Particles  [Space: spark burst]".to_string(),
        width: W as u32,
        height: H as u32,
        ..Config::default()
    })
    .run_with_update(
        init,
        // ── update ──────────────────────────────────────────────────────────
        |state, ctx, input, time| {
            state.elapsed += time.delta;

            let (view_proj, cam_right, cam_up) = camera_view_proj(state.elapsed);

            let fire_pos = Vec3::new(0.0, 0.0, 0.0);
            state
                .fire
                .update(time.delta, fire_pos, view_proj, cam_right, cam_up, ctx);

            state
                .sparks
                .update(time.delta, fire_pos, view_proj, cam_right, cam_up, ctx);

            if input.key_pressed(Key::Space) {
                state.sparks.burst(60, fire_pos + Vec3::Y * 0.5);
            }
        },
        // ── post-update (unused) ─────────────────────────────────────────────
        |_, _| {},
        // ── render ──────────────────────────────────────────────────────────
        |state, pass: &mut RenderPass| {
            state.sparks.draw(pass);
            state.fire.draw(pass);
        },
    );
}
