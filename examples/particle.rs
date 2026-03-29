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
    app::{App, Config, WindowId, run},
    input::{ActionMap, Input, Key},
    math::{Mat4, Vec3},
    particle::{EmitterConfig, ParticleSystem},
    renderer::{Context, RenderPass},
    time::Time,
};

const W: f32 = 960.0;
const H: f32 = 600.0;

fn camera_view_proj(time: f32) -> (Mat4, Vec3, Vec3) {
    let dist = 12.0_f32;
    let eye_x = dist * (time * 0.3).cos();
    let eye_z = dist * (time * 0.3).sin();
    let eye = Vec3::new(eye_x, 5.0, eye_z);
    let target = Vec3::new(0.0, 2.0, 0.0);
    let up = Vec3::Y;

    let view = Mat4::look_at_rh(eye, target, up);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, W / H, 0.1, 200.0);
    let view_proj = proj * view;

    let cam_right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
    let cam_up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);

    (view_proj, cam_right, cam_up)
}

#[derive(Hash, PartialEq, Eq)]
enum Action {
    SparkBurst,
}

struct ParticleDemo {
    elapsed: f32,
    dt: f32,
    view_proj: Mat4,
    cam_right: Vec3,
    cam_up: Vec3,
    burst_pending: bool,
    bindings: ActionMap<Action>,
    // GPU
    fire: Option<ParticleSystem>,
    sparks: Option<ParticleSystem>,
}

impl App for ParticleDemo {
    fn new() -> Self {
        let mut bindings = ActionMap::new();
        bindings.bind(Action::SparkBurst, Key::Space);
        ParticleDemo {
            elapsed: 0.0,
            dt: 0.0,
            view_proj: Mat4::IDENTITY,
            cam_right: Vec3::X,
            cam_up: Vec3::Y,
            burst_pending: false,
            bindings,
            fire: None,
            sparks: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.fire = Some(ParticleSystem::new(ctx, EmitterConfig::fire()));
        self.sparks = Some(ParticleSystem::new(ctx, EmitterConfig::sparks()));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        self.dt = time.delta;
        self.elapsed += time.delta;
        let (vp, right, up) = camera_view_proj(self.elapsed);
        self.view_proj = vp;
        self.cam_right = right;
        self.cam_up = up;

        if self.bindings.pressed(input, &Action::SparkBurst) {
            self.burst_pending = true;
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let fire_pos = Vec3::ZERO;
        let (vp, right, up) = (self.view_proj, self.cam_right, self.cam_up);

        if let Some(fire) = &mut self.fire {
            fire.update(self.dt, fire_pos, vp, right, up, ctx);
        }
        if let Some(sparks) = &mut self.sparks {
            sparks.update(self.dt, fire_pos, vp, right, up, ctx);
            if self.burst_pending {
                sparks.burst(60, fire_pos + Vec3::Y * 0.5);
            }
        }
        self.burst_pending = false;
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(sparks) = &self.sparks {
            sparks.draw(pass);
        }
        if let Some(fire) = &self.fire {
            fire.draw(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Particles  [Space: spark burst]",
            width: W as u32,
            height: H as u32,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<ParticleDemo>();
}
