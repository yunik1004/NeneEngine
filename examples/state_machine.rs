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
    animation::{
        AnimChannel, AnimState, Channel, Clip, Joint, JointMatrices, Skeleton, SkinnedMesh,
        SkinnedVertex, StateMachine, skinning_wgsl,
    },
    camera::Camera,
    input::Key,
    math::{Mat4, Quat, Vec3, Vec4},
    mesh::Model,
    renderer::{
        AMBIENT_LIGHT_WGSL, AmbientLight, Context, DIRECTIONAL_LIGHT_WGSL, DirectionalLight,
        IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    time::{Ease, Tween},
    ui::Ui,
    uniform,
    window::{Config, Window},
};

// ── Constants ─────────────────────────────────────────────────────────────────

const NUM_JOINTS: usize = 7;
const MAX_JOINTS: usize = 8;
const SIDES: usize = 14;
const RINGS: usize = (NUM_JOINTS - 1) * 2 + 1; // 13

const STATES: &[&str] = &["idle", "wave", "thrash"];
const BLEND_DURATION: f32 = 0.6;

// Easing options the user can cycle through for the crossfade.
const EASES: &[(Ease, &str)] = &[
    (Ease::Linear, "Linear"),
    (Ease::SineInOut, "SineInOut"),
    (Ease::CubicOut, "CubicOut"),
    (Ease::BackOut, "BackOut"),
    (Ease::ElasticOut, "ElasticOut"),
    (Ease::BounceOut, "BounceOut"),
];

// ── Shader ────────────────────────────────────────────────────────────────────

fn make_shader() -> String {
    format!(
        r#"
{skinning}
{ambient}
{directional}

struct SceneUniform {{
    view_proj:   mat4x4<f32>,
    model:       mat4x4<f32>,
    camera_pos:  vec4<f32>,
    ambient:     AmbientLight,
    directional: DirectionalLight,
}};
@group(0) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(0) var<uniform> joint_mats: JointMatrices;

struct VIn {{
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
    @location(3) joints:   vec4<u32>,
    @location(4) weights:  vec4<f32>,
}};
struct VOut {{
    @builtin(position) clip_pos:     vec4<f32>,
    @location(0)       world_pos:    vec3<f32>,
    @location(1)       world_normal: vec3<f32>,
    @location(2)       uv:           vec2<f32>,
}};

@vertex
fn vs_main(in: VIn) -> VOut {{
    let skin =
          in.weights.x * joint_mats.mats[in.joints.x]
        + in.weights.y * joint_mats.mats[in.joints.y]
        + in.weights.z * joint_mats.mats[in.joints.z]
        + in.weights.w * joint_mats.mats[in.joints.w];
    let world = scene.model * skin * vec4<f32>(in.position, 1.0);
    var out: VOut;
    out.clip_pos     = scene.view_proj * world;
    out.world_pos    = world.xyz;
    out.world_normal = normalize((scene.model * skin * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv           = in.uv;
    return out;
}}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {{
    let t = clamp(in.uv.y, 0.0, 1.0);
    let c0 = vec3<f32>(1.0,  0.28, 0.0);
    let c1 = vec3<f32>(0.18, 1.0,  0.3);
    let c2 = vec3<f32>(0.0,  0.75, 1.0);
    let albedo = select(
        mix(c0, c1, t * 2.0),
        mix(c1, c2, t * 2.0 - 1.0),
        t >= 0.5
    );
    let diffuse = directional_light(scene.directional, in.world_normal);
    let ambient_c = ambient_light(scene.ambient);
    let view_dir = normalize(scene.camera_pos.xyz - in.world_pos);
    let rim = pow(1.0 - max(dot(in.world_normal, view_dir), 0.0), 5.0) * 0.85;
    let rim_color = mix(albedo, vec3<f32>(0.6, 0.9, 1.0), 0.5) * rim;
    return vec4<f32>(albedo * (ambient_c + diffuse) + rim_color, 1.0);
}}
"#,
        skinning = skinning_wgsl(MAX_JOINTS),
        ambient = AMBIENT_LIGHT_WGSL,
        directional = DIRECTIONAL_LIGHT_WGSL,
    )
}

#[uniform]
struct SceneUniform {
    view_proj: Mat4,
    model: Mat4,
    camera_pos: Vec4,
    ambient: AmbientLight,
    directional: DirectionalLight,
}

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

fn build_mesh() -> SkinnedMesh {
    let mut vertices: Vec<SkinnedVertex> = Vec::with_capacity(RINGS * SIDES + 2);
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
            vertices.push(SkinnedVertex {
                position: [radius * c, y, radius * s],
                normal: [c, 0.0, s],
                uv: [i as f32 / SIDES as f32, t],
                joints: [j0, j1, 0, 0],
                weights: [w0, w1, 0.0, 0.0],
            });
        }
    }

    let base_cap = vertices.len() as u32;
    vertices.push(SkinnedVertex {
        position: [0.0, -0.1, 0.0],
        normal: [0.0, -1.0, 0.0],
        uv: [0.5, 0.0],
        joints: [0, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    });
    let tip_cap = vertices.len() as u32;
    let tip_y = (NUM_JOINTS - 1) as f32 + 0.35;
    vertices.push(SkinnedVertex {
        position: [0.0, tip_y, 0.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 1.0],
        joints: [(NUM_JOINTS - 1) as u8, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
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

    SkinnedMesh {
        vertices,
        indices,
        transform: Mat4::IDENTITY,
        base_color: None,
    }
}

fn build_model() -> Model {
    Model {
        meshes: vec![],
        skinned_meshes: vec![build_mesh()],
        skeleton: build_skeleton(),
        clips: vec![
            build_clip("idle", 0.08, 0.5),
            build_clip("wave", 0.35, 1.0),
            build_clip("thrash", 0.62, 2.0),
        ],
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ibuf: IndexBuffer,
    scene_buf: UniformBuffer,
    joint_buf: UniformBuffer,
    model: Model,
    sm: StateMachine,
    camera_angle: f32,
    ambient: AmbientLight,
    directional: DirectionalLight,
    next_state: usize,

    // Tween drives the crossfade blend weight with a selectable easing curve.
    blend_tween: Option<Tween<f32>>,
    ease_idx: usize,

    ui: Ui,
}

fn init(ctx: &mut Context) -> State {
    let model = build_model();
    let ambient = AmbientLight::new(Vec3::new(0.7, 0.75, 1.0), 0.18);
    let directional = DirectionalLight::new(
        Vec3::new(-1.0, -1.5, -0.8).normalize(),
        Vec3::new(1.0, 0.92, 0.8),
        1.1,
    );

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

    let joint_mats: JointMatrices<MAX_JOINTS> = sm.joint_buffer(&model.clips, &model.skeleton);
    let cam_pos = Vec3::new(9.0, 3.0, 0.0);
    let mut camera = Camera::perspective(cam_pos, 44.0, 0.1, 100.0);
    camera.target = Vec3::new(0.0, 3.0, 0.0);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let scene_buf = ctx.create_uniform_buffer(&SceneUniform {
        view_proj: camera.view_proj(aspect),
        model: Mat4::IDENTITY,
        camera_pos: cam_pos.extend(1.0),
        ambient,
        directional,
    });
    let joint_buf = ctx.create_uniform_buffer(&joint_mats);

    let mesh = &model.skinned_meshes[0];
    let vbuf = ctx.create_vertex_buffer(&mesh.vertices);
    let ibuf = ctx.create_index_buffer(&mesh.indices);

    let shader = make_shader();
    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(&shader, SkinnedVertex::layout())
            .with_uniform()
            .with_uniform()
            .with_depth(),
    );

    State {
        pipeline,
        vbuf,
        ibuf,
        scene_buf,
        joint_buf,
        model,
        sm,
        camera_angle: 0.0,
        ambient,
        directional,
        next_state: 1,
        blend_tween: None,
        ease_idx: 0,
        ui: Ui::new(ctx),
    }
}

fn main() {
    Window::new(Config {
        title: "State Machine — idle | wave | thrash   [Space: next  Q/E: ease]",
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            // ── Cycle easing function ─────────────────────────────────────────
            let n_eases = EASES.len();
            if input.key_pressed(Key::KeyE) {
                state.ease_idx = (state.ease_idx + 1) % n_eases;
            }
            if input.key_pressed(Key::KeyQ) {
                state.ease_idx = (state.ease_idx + n_eases - 1) % n_eases;
            }

            // ── Trigger next state ────────────────────────────────────────────
            if input.key_pressed(Key::Space) {
                let name = STATES[state.next_state];
                state.sm.trigger(name, BLEND_DURATION);
                state.next_state = (state.next_state + 1) % STATES.len();
                // Start a tween with the selected ease to track blend progress.
                state.blend_tween = Some(
                    Tween::new(0.0f32, 1.0, BLEND_DURATION).with_ease(EASES[state.ease_idx].0),
                );
            }

            // ── Advance tween ─────────────────────────────────────────────────
            let blend_progress = if let Some(ref mut t) = state.blend_tween {
                t.update(time.delta);
                let v = t.value();
                if t.is_done() {
                    state.blend_tween = None;
                }
                v
            } else {
                1.0
            };

            // ── Animation update ──────────────────────────────────────────────
            state.sm.update(time.delta, &state.model.clips);
            let joint_mats: JointMatrices<MAX_JOINTS> = state
                .sm
                .joint_buffer(&state.model.clips, &state.model.skeleton);
            ctx.update_uniform_buffer(&state.joint_buf, &joint_mats);

            // ── Camera orbit ──────────────────────────────────────────────────
            state.camera_angle += time.delta * 0.4;
            let r = 9.0;
            let cam_pos = Vec3::new(
                r * state.camera_angle.cos(),
                3.0,
                r * state.camera_angle.sin(),
            );
            let mut camera = Camera::perspective(cam_pos, 44.0, 0.1, 100.0);
            camera.target = Vec3::new(0.0, 3.0, 0.0);

            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            ctx.update_uniform_buffer(
                &state.scene_buf,
                &SceneUniform {
                    view_proj: camera.view_proj(aspect),
                    model: Mat4::IDENTITY,
                    camera_pos: cam_pos.extend(1.0),
                    ambient: state.ambient,
                    directional: state.directional,
                },
            );

            // ── UI: state + blend tween progress ─────────────────────────────
            let cur_name = STATES[(state.next_state + STATES.len() - 1) % STATES.len()];
            let ease_name = EASES[state.ease_idx].1;
            let bar = tween_bar(blend_progress);

            state
                .ui
                .begin_frame(input, cfg.width as f32, cfg.height as f32);
            state.ui.begin_panel("Animation", 16.0, 16.0, 200.0);
            state.ui.label(cur_name);
            state.ui.separator();
            state.ui.label_dim(&format!("ease: {ease_name}"));
            state.ui.label_dim(&bar);
            state.ui.separator();
            state.ui.label_dim("Space  next state");
            state.ui.label_dim("Q / E  cycle ease");
            state.ui.end_panel();
            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.scene_buf);
            pass.set_uniform(1, &state.joint_buf);
            pass.set_vertex_buffer(0, &state.vbuf);
            pass.draw_indexed_count(
                &state.ibuf,
                state.model.skinned_meshes[0].indices.len() as u32,
            );
            state.ui.render(pass);
        },
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Render blend progress as a text bar driven by the tween value.
fn tween_bar(t: f32) -> String {
    let filled = (t.clamp(0.0, 1.0) * 12.0) as usize;
    let empty = 12 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
