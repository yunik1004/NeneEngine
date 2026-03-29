//! Animation state machine demo — three clips, crossfade on Space.
//!
//! Procedural 7-joint serpent with skeletal animation.
//! State transitions use a [`Tween`] to drive the crossfade blend weight,
//! making the easing curve selectable at runtime.
//!
//! Controls
//! ──────────
//!   Space  — advance to next animation state
//!   Q / E  — cycle through easing functions for the next crossfade
//!
//! The panel shows the active ease, a live blend-progress bar, and the
//! current state name so you can feel the difference between e.g.
//! `Linear` and `ElasticOut` on the same transition.

use std::f32::consts::{PI, TAU};

use nene::{
    animation::{AnimChannel, AnimState, Channel, Clip, Joint, Mesh, Skeleton, StateMachine},
    app::{App, Config, WindowId, run},
    camera::Camera,
    input::{Input, Key},
    math::{Mat4, Quat, Vec2, Vec3, Vec4},
    mesh::{Model, Vertex},
    renderer::{
        AmbientLight, Context, DirectionalLight, GpuMesh, Material, MaterialBuilder, RenderPass,
    },
    time::{Ease, Time, Tween},
    ui::Ui,
};

// ── Constants ─────────────────────────────────────────────────────────────────

const NUM_JOINTS: usize = 7;
const SIDES: usize = 14;
const RINGS: usize = (NUM_JOINTS - 1) * 2 + 1;

const STATES: &[&str] = &["idle", "wave", "thrash"];
const BLEND_DURATION: f32 = 0.6;

const EASES: &[(Ease, &str)] = &[
    (Ease::Linear, "Linear"),
    (Ease::SineInOut, "SineInOut"),
    (Ease::CubicOut, "CubicOut"),
    (Ease::BackOut, "BackOut"),
    (Ease::ElasticOut, "ElasticOut"),
    (Ease::BounceOut, "BounceOut"),
];

// ── Procedural skeleton / mesh ────────────────────────────────────────────────

fn build_skeleton() -> Skeleton {
    let joints = (0..NUM_JOINTS)
        .map(|j| Joint {
            name: format!("j{j}"),
            parent: if j == 0 { None } else { Some(j - 1) },
            inverse_bind: Mat4::from_translation(Vec3::new(0.0, -(j as f32), 0.0)),
        })
        .collect();
    Skeleton { joints }
}

fn build_clip(name: &str, amplitude: f32, speed: f32) -> Clip {
    const FRAMES: usize = 40;
    const DURATION: f32 = 2.0;
    const PHASE_STEP: f32 = PI / 3.0;

    let times: Vec<f32> = (0..FRAMES)
        .map(|k| k as f32 * DURATION / FRAMES as f32)
        .collect();

    let mut channels: Vec<AnimChannel> = Vec::new();
    for j in 0..NUM_JOINTS {
        let amp = amplitude * (1.0 - 0.08 * j as f32).max(0.1);
        let values = times
            .iter()
            .map(|&t| {
                let angle = amp * (PI * t * speed + j as f32 * PHASE_STEP).sin();
                Quat::from_rotation_z(angle)
            })
            .collect();
        channels.push(AnimChannel::Rotation(Channel {
            joint: j,
            times: times.clone(),
            values,
        }));
    }
    for j in 1..NUM_JOINTS {
        channels.push(AnimChannel::Translation(Channel {
            joint: j,
            times: vec![0.0],
            values: vec![Vec3::new(0.0, 1.0, 0.0)],
        }));
    }
    Clip {
        name: name.into(),
        duration: DURATION,
        channels,
    }
}

fn build_mesh() -> Mesh {
    let mut vertices: Vec<Vertex> = Vec::with_capacity(RINGS * SIDES + 2);
    let mut indices: Vec<u32> = Vec::new();

    for k in 0..RINGS {
        let y = k as f32 * 0.5;
        let t = y / ((NUM_JOINTS - 1) as f32);
        let radius = 0.30 * (1.0 - t) + 0.045 * t;
        let (j0, j1, w0, w1) = if k % 2 == 0 {
            ((k / 2) as u8, 0u8, 1.0f32, 0.0f32)
        } else {
            ((k / 2) as u8, (k / 2 + 1) as u8, 0.5f32, 0.5f32)
        };
        for i in 0..SIDES {
            let angle = i as f32 * TAU / SIDES as f32;
            let (s, c) = angle.sin_cos();
            vertices.push(Vertex {
                position: Vec3::new(radius * c, y, radius * s),
                normal: Vec3::new(c, 0.0, s),
                uv: Vec2::new(i as f32 / SIDES as f32, t),
                joints: [j0, j1, 0, 0],
                weights: Vec4::new(w0, w1, 0.0, 0.0),
                ..Vertex::default()
            });
        }
    }

    let base_cap = vertices.len() as u32;
    vertices.push(Vertex {
        position: Vec3::new(0.0, -0.1, 0.0),
        normal: Vec3::NEG_Y,
        uv: Vec2::new(0.5, 0.0),
        joints: [0, 0, 0, 0],
        weights: Vec4::X,
        ..Vertex::default()
    });
    let tip_cap = vertices.len() as u32;
    let tip_y = (NUM_JOINTS - 1) as f32 + 0.35;
    vertices.push(Vertex {
        position: Vec3::new(0.0, tip_y, 0.0),
        normal: Vec3::Y,
        uv: Vec2::new(0.5, 1.0),
        joints: [(NUM_JOINTS - 1) as u8, 0, 0, 0],
        weights: Vec4::X,
        ..Vertex::default()
    });

    for k in 0..(RINGS - 1) {
        let base_k = (k * SIDES) as u32;
        let base_k1 = ((k + 1) * SIDES) as u32;
        for i in 0..SIDES as u32 {
            let next = (i + 1) % SIDES as u32;
            indices.extend_from_slice(&[
                base_k + i,
                base_k + next,
                base_k1 + next,
                base_k + i,
                base_k1 + next,
                base_k1 + i,
            ]);
        }
    }
    for i in 0..SIDES as u32 {
        let next = (i + 1) % SIDES as u32;
        indices.extend_from_slice(&[base_cap, next, i]);
    }
    let last_ring = ((RINGS - 1) * SIDES) as u32;
    for i in 0..SIDES as u32 {
        let next = (i + 1) % SIDES as u32;
        indices.extend_from_slice(&[tip_cap, last_ring + i, last_ring + next]);
    }

    let mut mesh = Mesh::new(vertices, indices);
    mesh.skinned = true;
    mesh
}

fn build_model() -> Model {
    Model {
        meshes: vec![build_mesh()],
        skeleton: build_skeleton(),
        clips: vec![
            build_clip("idle", 0.08, 0.5),
            build_clip("wave", 0.35, 1.0),
            build_clip("thrash", 0.62, 2.0),
        ],
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct StateMachineDemo {
    model: Model,
    gpu_mesh: Option<GpuMesh>,
    sm: StateMachine,
    camera_angle: f32,
    ambient: AmbientLight,
    directional: DirectionalLight,
    next_state: usize,
    blend_tween: Option<Tween<f32>>,
    ease_idx: usize,
    mat: Option<Material>,
    ui: Option<Ui>,
}

impl App for StateMachineDemo {
    fn new() -> Self {
        let model = build_model();
        let mut sm = StateMachine::new();
        for name in STATES {
            let clip_index = STATES.iter().position(|&s| s == *name).unwrap();
            sm.add_state(AnimState {
                name: name.to_string(),
                clip_index,
                looping: true,
                speed: 1.0,
            });
        }
        sm.current = 0;

        StateMachineDemo {
            model,
            gpu_mesh: None,
            sm,
            camera_angle: 0.0,
            ambient: AmbientLight::new(Vec3::new(0.7, 0.75, 1.0), 0.18),
            directional: DirectionalLight::new(
                Vec3::new(-1.0, -1.5, -0.8).normalize(),
                Vec3::new(1.0, 0.92, 0.8),
                1.1,
            ),
            next_state: 1,
            blend_tween: None,
            ease_idx: 0,
            mat: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let cam_pos = Vec3::new(9.0, 3.0, 0.0);
        let mut camera = Camera::perspective(cam_pos, 44.0, 0.1, 100.0);
        camera.target = Vec3::new(0.0, 3.0, 0.0);

        let (width, height) = {
            let cfg = ctx.surface_config();
            (cfg.width, cfg.height)
        };
        let aspect = width as f32 / height as f32;

        self.gpu_mesh = Some(GpuMesh::from_mesh(ctx, &self.model.meshes[0]));

        let mut mat = MaterialBuilder::new()
            .ambient()
            .directional()
            .rim()
            .skinned(self.model.skeleton.joints.len())
            .build(ctx);
        mat.uniform.color = Vec4::new(0.9, 0.6, 0.2, 1.0);
        mat.uniform.rim_color = Vec4::new(0.6, 0.9, 1.0, 1.0);
        mat.uniform.view_proj = camera.view_proj(aspect);
        mat.uniform.camera_pos = cam_pos.extend(1.0);
        mat.uniform.ambient = self.ambient;
        mat.uniform.directional = self.directional;
        mat.flush(ctx);
        self.mat = Some(mat);

        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        let n_eases = EASES.len();
        if input.key_pressed(Key::KeyE) {
            self.ease_idx = (self.ease_idx + 1) % n_eases;
        }
        if input.key_pressed(Key::KeyQ) {
            self.ease_idx = (self.ease_idx + n_eases - 1) % n_eases;
        }

        if input.key_pressed(Key::Space) {
            let name = STATES[self.next_state];
            self.sm.trigger(name, BLEND_DURATION);
            self.next_state = (self.next_state + 1) % STATES.len();
            self.blend_tween =
                Some(Tween::new(0.0f32, 1.0, BLEND_DURATION).with_ease(EASES[self.ease_idx].0));
        }

        if let Some(ref mut t) = self.blend_tween {
            t.update(time.delta);
            if t.is_done() {
                self.blend_tween = None;
            }
        }

        self.sm.update(time.delta, &self.model.clips);
        self.camera_angle += time.delta * 0.4;
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, input: &Input) {
        let joint_mats = self
            .sm
            .joint_matrices(&self.model.clips, &self.model.skeleton);
        if let Some(mat) = &self.mat {
            mat.update_joints(ctx, &joint_mats);
        }

        let r = 9.0;
        let cam_pos = Vec3::new(
            r * self.camera_angle.cos(),
            3.0,
            r * self.camera_angle.sin(),
        );
        let mut camera = Camera::perspective(cam_pos, 44.0, 0.1, 100.0);
        camera.target = Vec3::new(0.0, 3.0, 0.0);

        let (width, height) = {
            let cfg = ctx.surface_config();
            (cfg.width, cfg.height)
        };
        let aspect = width as f32 / height as f32;

        if let Some(mat) = &mut self.mat {
            mat.uniform.view_proj = camera.view_proj(aspect);
            mat.uniform.camera_pos = cam_pos.extend(1.0);
            mat.flush(ctx);
        }

        let blend_progress = self.blend_tween.as_ref().map_or(1.0, |t| t.value());
        let cur_name = STATES[(self.next_state + STATES.len() - 1) % STATES.len()];
        let ease_name = EASES[self.ease_idx].1;
        let bar = tween_bar(blend_progress);

        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, width as f32, height as f32);
        ui.begin_panel("Animation", 16.0, 16.0, 200.0);
        ui.label(cur_name);
        ui.separator();
        ui.label_dim(&format!("ease: {ease_name}"));
        ui.label_dim(&bar);
        ui.separator();
        ui.label_dim("Space  next state");
        ui.label_dim("Q / E  cycle ease");
        ui.end_panel();
        ui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let (Some(mat), Some(mesh)) = (&self.mat, &self.gpu_mesh) {
            mat.render(pass, mesh);
        }
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "State Machine — idle | wave | thrash   [Space: next  Q/E: ease]",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<StateMachineDemo>();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tween_bar(t: f32) -> String {
    let filled = (t.clamp(0.0, 1.0) * 12.0) as usize;
    let empty = 12 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
