//! Animation state machine demo — three clips, crossfade on Space.
//!
//! Procedural 7-joint serpent (same mesh as `animation`).
//!
//! States
//! ──────
//!   idle   — barely-breathing sway  (low amplitude, slow)
//!   wave   — travelling sine ripple (medium amplitude)
//!   thrash — frantic thrash         (high amplitude, fast)
//!
//! Press Space to advance to the next state with a 0.4 s crossfade.
//! The title bar shows the active state and blend progress.

use std::f32::consts::{PI, TAU};

use nene::{
    animation::{
        AnimChannel, AnimState, Channel, Clip, Joint, JointMatrices, Skeleton, SkinnedMesh,
        SkinnedVertex, StateMachine, skinning_wgsl,
    },
    camera::Camera,
    input::Key,
    light::{AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight},
    math::{Mat4, Quat, Vec3, Vec4},
    mesh::Model,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    uniform,
    window::{Config, Window},
};

// ── Constants ─────────────────────────────────────────────────────────────────

const NUM_JOINTS: usize = 7;
const MAX_JOINTS: usize = 8;
const SIDES: usize = 14;
const RINGS: usize = (NUM_JOINTS - 1) * 2 + 1; // 13

// State names (also the cycle order).
const STATES: &[&str] = &["idle", "wave", "thrash"];
const BLEND_DURATION: f32 = 0.4;

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
    // Body colour gradient: coral → lime → sky
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

/// Build a sinusoidal clip with the given amplitude and speed.
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

fn build_model() -> Model {
    Model {
        meshes: vec![],
        skinned_meshes: vec![build_mesh()],
        skeleton: build_skeleton(),
        clips: vec![
            build_clip("idle", 0.08, 0.5),   // barely breathing
            build_clip("wave", 0.35, 1.0),   // medium ripple
            build_clip("thrash", 0.62, 2.0), // frantic thrash
        ],
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

    let base_cap_idx = vertices.len() as u32;
    vertices.push(SkinnedVertex {
        position: [0.0, -0.1, 0.0],
        normal: [0.0, -1.0, 0.0],
        uv: [0.5, 0.0],
        joints: [0, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    });
    let tip_cap_idx = vertices.len() as u32;
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
        indices.extend_from_slice(&[base_cap_idx, next, i]);
    }
    let last_ring = ((RINGS - 1) * SIDES) as u32;
    for i in 0..SIDES as u32 {
        let next = (i + 1) % SIDES as u32;
        indices.extend_from_slice(&[tip_cap_idx, last_ring + i, last_ring + next]);
    }

    SkinnedMesh {
        vertices,
        indices,
        transform: Mat4::IDENTITY,
        base_color: None,
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
    /// Index into STATES for the next trigger.
    next_state: usize,
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
    // Start in "idle".
    sm.current = 0;

    let joint_mats: JointMatrices<MAX_JOINTS> = sm.joint_buffer(&model.clips, &model.skeleton);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let cam_pos = Vec3::new(9.0, 3.0, 0.0);
    let mut camera = Camera::perspective(cam_pos, 44.0, 0.1, 100.0);
    camera.target = Vec3::new(0.0, 3.0, 0.0);

    let scene_uniform = SceneUniform {
        view_proj: camera.view_proj(aspect),
        model: Mat4::IDENTITY,
        camera_pos: cam_pos.extend(1.0),
        ambient,
        directional,
    };

    let scene_buf = ctx.create_uniform_buffer(&scene_uniform);
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
        next_state: 1, // Space will trigger "wave" first
    }
}

fn main() {
    Window::new(Config {
        title: "State Machine — idle | wave | thrash   [Space: next state]".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            // Trigger next state on Space press.
            if input.key_pressed(Key::Space) {
                let name = STATES[state.next_state];
                state.sm.trigger(name, BLEND_DURATION);
                state.next_state = (state.next_state + 1) % STATES.len();
            }

            state.sm.update(time.delta, &state.model.clips);

            let joint_mats: JointMatrices<MAX_JOINTS> = state
                .sm
                .joint_buffer(&state.model.clips, &state.model.skeleton);
            ctx.update_uniform_buffer(&state.joint_buf, &joint_mats);

            // Orbit camera.
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
        },
    );
}
