//! GPU instancing demo — 2 500 cubes, one draw call.
//!
//! Per-vertex data  (slot 0): position + normal
//! Per-instance data (slot 1): world offset + color   ← InstanceBuffer
//!
//! Everything is packed into a single `draw_indexed_instanced` call.
//! The title bar shows live instance count and approximate draw calls.

use nene::{
    math::{Mat4, Vec3, Vec4},
    renderer::{
        Context, IndexBuffer, InstanceBuffer, Pipeline, PipelineDescriptor, RenderPass,
        UniformBuffer, VertexBuffer,
    },
    uniform, vertex,
    window::{Config, Window},
};

// ── Grid ──────────────────────────────────────────────────────────────────────

const GRID: i32 = 50; // GRID × GRID cubes
const SPACING: f32 = 2.2;

// ── Shader ────────────────────────────────────────────────────────────────────

const SHADER: &str = r#"
struct Scene {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,   // .xyz = direction (normalised), .w unused
    time:      vec4<f32>,   // .x = seconds
}
@group(0) @binding(0) var<uniform> scene: Scene;

struct VIn {
    // per-vertex (slot 0)
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    // per-instance (slot 1)
    @location(2) offset:   vec3<f32>,
    @location(3) color:    vec4<f32>,
}
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color:  vec4<f32>,
}

@vertex fn vs_main(v: VIn) -> VOut {
    // Each cube spins at a speed that varies by position so they're all different.
    let speed = 0.4 + 0.6 * fract(v.offset.x * 0.17 + v.offset.z * 0.13);
    let angle = scene.time.x * speed;
    let c = cos(angle); let s = sin(angle);
    // Rotation around Y axis.
    let rotated = vec3<f32>(
        c * v.position.x + s * v.position.z,
        v.position.y,
       -s * v.position.x + c * v.position.z,
    );
    let world = rotated + v.offset;

    var out: VOut;
    out.clip   = scene.view_proj * vec4<f32>(world, 1.0);
    out.normal = normalize(vec3<f32>(c * v.normal.x + s * v.normal.z,
                                     v.normal.y,
                                    -s * v.normal.x + c * v.normal.z));
    out.color  = v.color;
    return out;
}

@fragment fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let diffuse  = max(dot(in.normal, -scene.light_dir.xyz), 0.0);
    let ambient  = 0.25;
    let lit      = in.color.rgb * (ambient + diffuse * 0.75);
    return vec4<f32>(lit, 1.0);
}
"#;

// ── Types ─────────────────────────────────────────────────────────────────────

#[vertex]
struct Vtx {
    pos: [f32; 3],
    normal: [f32; 3],
}

/// Per-instance: world offset + RGBA color (locations shifted to 2, 3).
#[vertex]
struct Inst {
    offset: [f32; 3],
    color: [f32; 4],
}

#[uniform]
struct Scene {
    view_proj: Mat4,
    light_dir: Vec4,
    time: Vec4,
}

// ── Cube geometry ─────────────────────────────────────────────────────────────

fn cube() -> (Vec<Vtx>, Vec<u32>) {
    // 6 faces × 4 vertices, indexed with 2 triangles per face.
    let faces: [([f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 1.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, -1.0], [0.0, 0.0, -1.0]),
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, -1.0, 0.0], [0.0, -1.0, 0.0]),
    ];

    let verts_per_face: [[f32; 3]; 4] = [
        [-1.0, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [1.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
    ];

    let mut verts: Vec<Vtx> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    for (face_normal, normal) in &faces {
        let n = *normal;
        // Build a local basis from the face normal so we can orient the quad.
        let up = if n[1].abs() < 0.9 {
            [0.0_f32, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let right = cross(n, up);
        let up2 = cross(right, n);

        let base = verts.len() as u32;
        for lp in &verts_per_face {
            let p = [
                face_normal[0] + lp[0] * right[0] + lp[1] * up2[0],
                face_normal[1] + lp[0] * right[1] + lp[1] * up2[1],
                face_normal[2] + lp[0] * right[2] + lp[1] * up2[2],
            ];
            verts.push(Vtx { pos: p, normal: n });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (verts, idx)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ibuf: IndexBuffer,
    inst_buf: InstanceBuffer,
    scene_buf: UniformBuffer,
    instances: Vec<Inst>,
}

fn build_instances() -> Vec<Inst> {
    let mut out = Vec::new();
    for iz in 0..GRID {
        for ix in 0..GRID {
            let x = (ix - GRID / 2) as f32 * SPACING;
            let z = (iz - GRID / 2) as f32 * SPACING;
            // Hue from position angle around the grid centre.
            let hue = (ix as f32 / GRID as f32 + iz as f32 / GRID as f32) % 1.0;
            let color = hsv_to_rgb(hue, 0.75, 0.9);
            out.push(Inst {
                offset: [x, 0.0, z],
                color: [color[0], color[1], color[2], 1.0],
            });
        }
    }
    out
}

/// Tiny HSV→RGB (h in 0..1).
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0) as u32;
    let f = h * 6.0 - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

fn init(ctx: &mut Context) -> State {
    let (verts, indices) = cube();
    let instances = build_instances();

    let vbuf = ctx.create_vertex_buffer(&verts);
    let ibuf = ctx.create_index_buffer(&indices);
    let inst_buf = ctx.create_instance_buffer(&instances);

    let scene_buf = ctx.create_uniform_buffer(&Scene {
        view_proj: Mat4::IDENTITY,
        light_dir: Vec4::new(-0.4, -0.7, -0.5, 0.0).normalize(),
        time: Vec4::ZERO,
    });

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, Vtx::layout())
            .with_instance_layout(Inst::layout().at_locations(2))
            .with_uniform()
            .with_depth(),
    );

    State {
        pipeline,
        vbuf,
        ibuf,
        inst_buf,
        scene_buf,
        instances,
    }
}

fn main() {
    Window::new(Config {
        title: "Instancing — 1 draw call",
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, _input, time| {
            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            let t = time.elapsed as f32;

            // Slowly orbit camera.
            let r = GRID as f32 * SPACING * 0.65;
            let cam_pos = Vec3::new(r * (t * 0.12).cos(), r * 0.45, r * (t * 0.12).sin());
            let view_proj = Mat4::perspective_rh(45_f32.to_radians(), aspect, 0.5, r * 3.0)
                * Mat4::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);

            ctx.update_uniform_buffer(
                &state.scene_buf,
                &Scene {
                    view_proj,
                    light_dir: Vec4::new(-0.4, -0.7, -0.5, 0.0).normalize(),
                    time: Vec4::new(t, 0.0, 0.0, 0.0),
                },
            );
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.scene_buf);
            pass.set_vertex_buffer(0, &state.vbuf);
            pass.set_instance_buffer(1, &state.inst_buf);
            pass.draw_indexed_instanced(&state.ibuf, state.instances.len() as u32);
        },
    );
}
