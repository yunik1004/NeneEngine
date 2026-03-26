//! "Neon Serpent" — procedural 7-joint skeletal animation.
//!
//! A tapered cylinder is deformed by a traveling sine-wave ripple.
//! Everything is built in code (no external files needed).
//! The camera orbits slowly around the creature.

use std::f32::consts::{PI, TAU};

use nene::{
    animation::{
        AnimChannel, AnimatedModel, Animator, Channel, Clip, Joint, JointMatrices, Skeleton,
        SkinnedMesh, SkinnedVertex, skinning_wgsl,
    },
    camera::Camera,
    light::{AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight},
    math::{Mat4, Quat, Vec3, Vec4},
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    uniform,
    window::{Config, Window},
};

// ── Constants ─────────────────────────────────────────────────────────────────

const NUM_JOINTS: usize = 7;
/// Must be ≥ NUM_JOINTS. Padded so the WGSL array aligns cleanly.
const MAX_JOINTS: usize = 8;
/// Cylinder cross-section sides — more = smoother.
const SIDES: usize = 14;
/// Total rings = one per joint + one between each adjacent pair.
const RINGS: usize = (NUM_JOINTS - 1) * 2 + 1; // 13

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
    @builtin(position) clip_pos:    vec4<f32>,
    @location(0)       world_pos:   vec3<f32>,
    @location(1)       world_normal: vec3<f32>,
    @location(2)       uv:          vec2<f32>,
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
    // 3-stop body gradient along the spine: orange → lime → cyan
    let t = clamp(in.uv.y, 0.0, 1.0);
    let c0 = vec3<f32>(1.0,  0.28, 0.0);  // deep orange (base)
    let c1 = vec3<f32>(0.18, 1.0,  0.3);  // lime green  (mid)
    let c2 = vec3<f32>(0.0,  0.75, 1.0);  // cyan        (tip)
    let albedo = select(
        mix(c0, c1, t * 2.0),
        mix(c1, c2, t * 2.0 - 1.0),
        t >= 0.5
    );

    // Diffuse + ambient
    let diffuse  = directional_light(scene.directional, in.world_normal);
    let ambient_c = ambient_light(scene.ambient);

    // Fresnel rim glow — brightest at silhouette edges
    let view_dir = normalize(scene.camera_pos.xyz - in.world_pos);
    let nv  = max(dot(in.world_normal, view_dir), 0.0);
    let rim = pow(1.0 - nv, 5.0) * 0.85;
    // Rim tint matches body segment color, slightly shifted toward white
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

// ── Procedural model ──────────────────────────────────────────────────────────

fn build_model() -> AnimatedModel {
    AnimatedModel {
        meshes: vec![build_mesh()],
        skeleton: build_skeleton(),
        clips: vec![build_clip()],
    }
}

/// 7 joints in a chain along +Y (j0 at y=0, j6 at y=6).
fn build_skeleton() -> Skeleton {
    let joints = (0..NUM_JOINTS)
        .map(|j| Joint {
            name: format!("j{j}"),
            parent: if j == 0 { None } else { Some(j - 1) },
            // IBM transforms from world bind-pose space into joint j's local space.
            inverse_bind: Mat4::from_translation(Vec3::new(0.0, -(j as f32), 0.0)),
        })
        .collect();
    Skeleton { joints }
}

/// Sinusoidal traveling-wave clip: each joint rotates around Z with a 60° phase offset.
fn build_clip() -> Clip {
    const FRAMES: usize = 40;
    const DURATION: f32 = 2.0;
    // amplitude decreases slightly toward the tip for a more natural look
    let amplitudes = [0.38f32, 0.36, 0.33, 0.30, 0.26, 0.22, 0.18];
    const PHASE_STEP: f32 = PI / 3.0; // 60° between consecutive joints

    // Keyframe times span [0, DURATION) — the animator loops by rem_euclid.
    let times: Vec<f32> = (0..FRAMES)
        .map(|k| k as f32 * DURATION / FRAMES as f32)
        .collect();

    let mut channels: Vec<AnimChannel> = Vec::new();

    // Rotation channels — traveling sine-wave
    for j in 0..NUM_JOINTS {
        let amp = amplitudes[j];
        let values = times
            .iter()
            .map(|&t| {
                let angle = amp * (PI * t + j as f32 * PHASE_STEP).sin();
                Quat::from_rotation_z(angle)
            })
            .collect();
        channels.push(AnimChannel::Rotation(Channel {
            joint: j,
            times: times.clone(),
            values,
        }));
    }

    // Constant translation channels for non-root joints.
    // compute_joint_matrices() starts from JointPose::IDENTITY (T=ZERO),
    // so without these the entire chain collapses to y=0.
    for j in 1..NUM_JOINTS {
        channels.push(AnimChannel::Translation(Channel {
            joint: j,
            times: vec![0.0],
            values: vec![Vec3::new(0.0, 1.0, 0.0)],
        }));
    }

    Clip {
        name: "wave".into(),
        duration: DURATION,
        channels,
    }
}

/// Tapered 14-sided cylinder with smooth blend weights at segment boundaries.
fn build_mesh() -> SkinnedMesh {
    let mut vertices: Vec<SkinnedVertex> = Vec::with_capacity(RINGS * SIDES + 2);
    let mut indices: Vec<u32> = Vec::new();

    // ── Rings ────────────────────────────────────────────────────────────────
    for k in 0..RINGS {
        let y = k as f32 * 0.5;
        let t = y / ((NUM_JOINTS - 1) as f32); // 0..1 along the body
        let radius = 0.30 * (1.0 - t) + 0.045 * t; // taper

        // Even ring → sits exactly on joint k/2.
        // Odd ring → halfway between joints k/2 and k/2+1 (blend 50/50).
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

    // ── Cap centers ──────────────────────────────────────────────────────────
    let base_cap_idx = vertices.len() as u32;
    vertices.push(SkinnedVertex {
        position: [0.0, -0.1, 0.0], // slight inset so the base looks flat
        normal: [0.0, -1.0, 0.0],
        uv: [0.5, 0.0],
        joints: [0, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    });

    let tip_cap_idx = vertices.len() as u32;
    let tip_y = (NUM_JOINTS - 1) as f32 + 0.35; // protrude a little past the last joint
    vertices.push(SkinnedVertex {
        position: [0.0, tip_y, 0.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 1.0],
        joints: [(NUM_JOINTS - 1) as u8, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    });

    // ── Body quads ───────────────────────────────────────────────────────────
    for k in 0..(RINGS - 1) {
        let base_k = (k * SIDES) as u32;
        let base_k1 = ((k + 1) * SIDES) as u32;
        for i in 0..SIDES as u32 {
            let next = (i + 1) % SIDES as u32;
            let v00 = base_k + i;
            let v10 = base_k + next;
            let v01 = base_k1 + i;
            let v11 = base_k1 + next;
            // CCW winding viewed from outside the cylinder
            indices.extend_from_slice(&[v00, v10, v11, v00, v11, v01]);
        }
    }

    // ── Base cap (normal faces −Y) ────────────────────────────────────────────
    for i in 0..SIDES as u32 {
        let next = (i + 1) % SIDES as u32;
        // CCW from below (−Y view) = reversed winding from above
        indices.extend_from_slice(&[base_cap_idx, next, i]);
    }

    // ── Tip cap (normal faces +Y) ────────────────────────────────────────────
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
    model: AnimatedModel,
    animator: Animator,
    camera_angle: f32,
    ambient: AmbientLight,
    directional: DirectionalLight,
}

fn init(ctx: &mut Context) -> State {
    let model = build_model();
    let clip = model.clips.first().unwrap();

    let ambient = AmbientLight::new(Vec3::new(0.7, 0.75, 1.0), 0.18);
    let directional = DirectionalLight::new(
        Vec3::new(-1.0, -1.5, -0.8).normalize(),
        Vec3::new(1.0, 0.92, 0.8),
        1.1,
    );

    let animator = Animator::new();
    let joint_mats: JointMatrices<MAX_JOINTS> = animator.joint_buffer(clip, &model.skeleton);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let cam_pos = Vec3::new(9.0, 3.0, 0.0);
    let mut camera = Camera::perspective(cam_pos, 44.0, 0.1, 100.0);
    camera.target = Vec3::new(0.0, 3.0, 0.0); // look at the body center

    let scene_uniform = SceneUniform {
        view_proj: camera.view_proj(aspect),
        model: Mat4::IDENTITY,
        camera_pos: cam_pos.extend(1.0),
        ambient,
        directional,
    };

    let scene_buf = ctx.create_uniform_buffer(&scene_uniform);
    let joint_buf = ctx.create_uniform_buffer(&joint_mats);

    let mesh = &model.meshes[0];
    let vbuf = ctx.create_vertex_buffer(&mesh.vertices);
    let ibuf = ctx.create_index_buffer(&mesh.indices);

    let shader = make_shader();
    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(&shader, SkinnedVertex::layout())
            .with_uniform() // group 0: scene
            .with_uniform() // group 1: joint matrices
            .with_depth(),
    );

    State {
        pipeline,
        vbuf,
        ibuf,
        scene_buf,
        joint_buf,
        model,
        animator,
        camera_angle: 0.0,
        ambient,
        directional,
    }
}

fn main() {
    Window::new(Config {
        title: "Neon Serpent — skeletal animation".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, _input, time| {
            let clip = state.model.clips.first().unwrap();
            state.animator.update(time.delta, clip);

            // Upload new joint matrices
            let joint_mats: JointMatrices<MAX_JOINTS> =
                state.animator.joint_buffer(clip, &state.model.skeleton);
            ctx.update_uniform_buffer(&state.joint_buf, &joint_mats);

            // Orbit camera in the XZ plane at body-center height (y=3)
            state.camera_angle += time.delta * 0.45;
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
            pass.draw_indexed_count(&state.ibuf, state.model.meshes[0].indices.len() as u32);
        },
    );
}
