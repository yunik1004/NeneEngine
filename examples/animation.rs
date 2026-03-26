/// Skeletal animation example.
///
/// Loads a skinned glTF/GLB and plays its first animation clip.
///
/// Usage: cargo run --example animation -- path/to/model.glb
/// (argument optional — defaults to a built-in two-bone arm)
use nene::{
    animation::SkinnedVertex,
    animation::{AnimatedModel, Animator, JointMatrices, skinning_wgsl},
    camera::Camera,
    light::{AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight},
    math::{Mat4, Vec3},
    renderer::{
        Context, FilterMode, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, Texture,
        UniformBuffer, VertexBuffer,
    },
    uniform,
    window::{Config, Window},
};

const MAX_JOINTS: usize = 64;

fn make_shader() -> String {
    format!(
        r#"
{skinning}
{ambient}
{directional}

struct SceneUniform {{
    view_proj:  mat4x4<f32>,
    model:      mat4x4<f32>,
    ambient:    AmbientLight,
    directional: DirectionalLight,
}};
@group(0) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(0) var<uniform> joint_mats: JointMatrices;
@group(2) @binding(0) var t_color: texture_2d<f32>;
@group(2) @binding(1) var s_color: sampler;

struct VIn {{
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
    @location(3) joints:   vec4<u32>,
    @location(4) weights:  vec4<f32>,
}};
struct VOut {{
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_normal:   vec3<f32>,
    @location(1) uv:             vec2<f32>,
}};

@vertex
fn vs_main(in: VIn) -> VOut {{
    let skin =
          in.weights.x * joint_mats.mats[in.joints.x]
        + in.weights.y * joint_mats.mats[in.joints.y]
        + in.weights.z * joint_mats.mats[in.joints.z]
        + in.weights.w * joint_mats.mats[in.joints.w];
    let world_pos = scene.model * skin * vec4<f32>(in.position, 1.0);
    var out: VOut;
    out.clip_pos    = scene.view_proj * world_pos;
    out.world_normal = normalize((scene.model * skin * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv           = in.uv;
    return out;
}}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {{
    let albedo  = textureSample(t_color, s_color, in.uv);
    let diffuse = directional_light(scene.directional, in.world_normal);
    let ambient = ambient_light(scene.ambient);
    return vec4<f32>(albedo.rgb * (ambient + diffuse), albedo.a);
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
    ambient: AmbientLight,
    directional: DirectionalLight,
}

struct MeshGpu {
    vbuf: VertexBuffer,
    ibuf: IndexBuffer,
    texture: Texture,
}

struct State {
    pipeline: Pipeline,
    meshes: Vec<MeshGpu>,
    scene_buf: UniformBuffer,
    joint_buf: UniformBuffer,
    model: AnimatedModel,
    animator: Animator,
    ambient: AmbientLight,
    directional: DirectionalLight,
}

fn init(ctx: &mut Context, path: &str) -> State {
    let model = AnimatedModel::load(path)
        .unwrap_or_else(|| panic!("'{}' has no skin — use a skinned glTF/GLB", path));

    let ambient = AmbientLight::new(Vec3::ONE, 0.2);
    let directional =
        DirectionalLight::new(Vec3::new(1.0, -2.0, -1.0), Vec3::new(1.0, 0.95, 0.9), 1.0);

    let animator = Animator::new();
    let clip = model.clips.first().expect("model has no animation clips");
    let joint_mats: JointMatrices<MAX_JOINTS> = animator.joint_buffer(clip, &model.skeleton);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let camera = Camera::perspective(Vec3::new(0.0, 1.0, 3.0), 45.0, 0.1, 100.0);
    let scene_uniform = SceneUniform {
        view_proj: camera.view_proj(aspect),
        model: Mat4::IDENTITY,
        ambient,
        directional,
    };

    let scene_buf = ctx.create_uniform_buffer(&scene_uniform);
    let joint_buf = ctx.create_uniform_buffer(&joint_mats);

    let shader = make_shader();
    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(&shader, SkinnedVertex::layout())
            .with_uniform()
            .with_uniform()
            .with_texture()
            .with_depth(),
    );

    let meshes = model
        .meshes
        .iter()
        .map(|mesh| {
            let vbuf = ctx.create_vertex_buffer(&mesh.vertices);
            let ibuf = ctx.create_index_buffer(&mesh.indices);
            let texture = match &mesh.base_color {
                Some(img) => {
                    ctx.create_texture_with(img.width, img.height, &img.data, FilterMode::Linear)
                }
                None => ctx.create_texture_with(1, 1, &[255, 255, 255, 255], FilterMode::Nearest),
            };
            MeshGpu {
                vbuf,
                ibuf,
                texture,
            }
        })
        .collect();

    State {
        pipeline,
        meshes,
        scene_buf,
        joint_buf,
        model,
        animator,
        ambient,
        directional,
    }
}

fn main() {
    let tmp;
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            tmp = write_sample_glb();
            tmp.to_str().unwrap().to_string()
        }
    };

    Window::new(Config {
        title: "Animation".to_string(),
        ..Config::default()
    })
    .run_with_update(
        move |ctx| init(ctx, &path),
        |state, ctx, _input, time| {
            let clip = state.model.clips.first().unwrap();
            state.animator.update(time.delta, clip);

            let joint_mats: JointMatrices<MAX_JOINTS> =
                state.animator.joint_buffer(clip, &state.model.skeleton);
            ctx.update_uniform_buffer(&state.joint_buf, &joint_mats);

            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            let camera = Camera::perspective(Vec3::new(0.0, 1.0, 3.0), 45.0, 0.1, 100.0);
            ctx.update_uniform_buffer(
                &state.scene_buf,
                &SceneUniform {
                    view_proj: camera.view_proj(aspect),
                    model: Mat4::IDENTITY,
                    ambient: state.ambient,
                    directional: state.directional,
                },
            );
        },
        |_state, _ctx| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            for mesh in &state.meshes {
                pass.set_uniform(0, &state.scene_buf);
                pass.set_uniform(1, &state.joint_buf);
                pass.set_texture(2, &mesh.texture);
                pass.set_vertex_buffer(0, &mesh.vbuf);
                pass.draw_indexed(&mesh.ibuf);
            }
        },
    );
}

// ── Built-in sample: two-bone arm that bends at the elbow ─────────────────────

fn write_sample_glb() -> std::path::PathBuf {
    // 4-vertex quad deformed by 2 joints.
    //   joint 0 (root): fixed at origin
    //   joint 1 (child): animates rotation around Z so the top half waves
    //
    // Vertices:
    //   0 (-0.2, 0.0)  — fully joint 0
    //   1 ( 0.2, 0.0)  — fully joint 0
    //   2 (-0.2, 1.0)  — fully joint 1
    //   3 ( 0.2, 1.0)  — fully joint 1
    let positions: &[[f32; 3]] = &[
        [-0.2, 0.0, 0.0],
        [0.2, 0.0, 0.0],
        [-0.2, 1.0, 0.0],
        [0.2, 1.0, 0.0],
    ];
    let normals: &[[f32; 3]] = &[
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];
    let uvs: &[[f32; 2]] = &[[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
    // joints: each vertex influenced by one joint
    let joints_u8: &[[u8; 4]] = &[[0, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]];
    let weights: &[[f32; 4]] = &[
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let indices: &[u16] = &[0, 1, 2, 1, 3, 2];

    // Inverse bind matrices: identity for both joints
    // (bind pose = world origin)
    let ibm: [[f32; 16]; 2] = [
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 1.0,
        ],
    ];

    // Animation: joint 1 rotates around Z from 0 to ~45° and back
    // 3 keyframes at t = 0.0, 0.5, 1.0
    // Rotations as [x, y, z, w] quaternions
    let rot_times: &[f32] = &[0.0, 0.5, 1.0];
    let rot_values: &[[f32; 4]] = &[
        [0.0, 0.0, 0.0, 1.0],     // 0°
        [0.0, 0.0, 0.383, 0.924], // ~45°
        [0.0, 0.0, 0.0, 1.0],     // back to 0°
    ];

    let mut buf: Vec<u8> = Vec::new();

    let pos_off = buf.len();
    for p in positions {
        for &f in p {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let pos_len = buf.len() - pos_off;

    let nor_off = buf.len();
    for n in normals {
        for &f in n {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let nor_len = buf.len() - nor_off;

    let uv_off = buf.len();
    for uv in uvs {
        for &f in uv {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let uv_len = buf.len() - uv_off;

    // joints as u8 (component type 5121)
    let jnt_off = buf.len();
    for j in joints_u8 {
        buf.extend_from_slice(j);
    }
    let jnt_len = buf.len() - jnt_off;

    let wgt_off = buf.len();
    for w in weights {
        for &f in w {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let wgt_len = buf.len() - wgt_off;

    let idx_off = buf.len();
    for &i in indices {
        buf.extend_from_slice(&i.to_le_bytes());
    }
    let idx_len = buf.len() - idx_off;

    let ibm_off = buf.len();
    for mat in &ibm {
        for &f in mat {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let ibm_len = buf.len() - ibm_off;

    let anim_t_off = buf.len();
    for &t in rot_times {
        buf.extend_from_slice(&t.to_le_bytes());
    }
    let anim_t_len = buf.len() - anim_t_off;

    let anim_v_off = buf.len();
    for q in rot_values {
        for &f in q {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let anim_v_len = buf.len() - anim_v_off;

    let total = buf.len();
    let b64 = base64_encode(&buf);

    let json = format!(
        r#"{{
  "asset": {{"version":"2.0"}},
  "scene": 0,
  "scenes": [{{"nodes":[0]}}],
  "nodes": [
    {{"name":"root","mesh":0,"skin":0,"children":[1]}},
    {{"name":"joint1","translation":[0.0,1.0,0.0]}}
  ],
  "meshes": [{{"primitives":[{{"attributes":{{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2,"JOINTS_0":3,"WEIGHTS_0":4}},"indices":5}}]}}],
  "skins": [{{"joints":[0,1],"inverseBindMatrices":6,"skeleton":0}}],
  "animations": [{{"name":"wave","channels":[
    {{"sampler":0,"target":{{"node":1,"path":"rotation"}}}}
  ],"samplers":[
    {{"input":7,"output":8,"interpolation":"LINEAR"}}
  ]}}],
  "accessors": [
    {{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3","min":[-0.2,0.0,0.0],"max":[0.2,1.0,0.0]}},
    {{"bufferView":1,"componentType":5126,"count":4,"type":"VEC3"}},
    {{"bufferView":2,"componentType":5126,"count":4,"type":"VEC2"}},
    {{"bufferView":3,"componentType":5121,"count":4,"type":"VEC4"}},
    {{"bufferView":4,"componentType":5126,"count":4,"type":"VEC4"}},
    {{"bufferView":5,"componentType":5123,"count":6,"type":"SCALAR"}},
    {{"bufferView":6,"componentType":5126,"count":2,"type":"MAT4"}},
    {{"bufferView":7,"componentType":5126,"count":3,"type":"SCALAR","min":[0.0],"max":[1.0]}},
    {{"bufferView":8,"componentType":5126,"count":3,"type":"VEC4"}}
  ],
  "bufferViews": [
    {{"buffer":0,"byteOffset":{pos_off},"byteLength":{pos_len}}},
    {{"buffer":0,"byteOffset":{nor_off},"byteLength":{nor_len}}},
    {{"buffer":0,"byteOffset":{uv_off},"byteLength":{uv_len}}},
    {{"buffer":0,"byteOffset":{jnt_off},"byteLength":{jnt_len}}},
    {{"buffer":0,"byteOffset":{wgt_off},"byteLength":{wgt_len}}},
    {{"buffer":0,"byteOffset":{idx_off},"byteLength":{idx_len}}},
    {{"buffer":0,"byteOffset":{ibm_off},"byteLength":{ibm_len},"byteStride":64}},
    {{"buffer":0,"byteOffset":{anim_t_off},"byteLength":{anim_t_len}}},
    {{"buffer":0,"byteOffset":{anim_v_off},"byteLength":{anim_v_len}}}
  ],
  "buffers": [{{"byteLength":{total},"uri":"data:application/octet-stream;base64,{b64}"}}]
}}"#
    );

    let path = std::env::temp_dir().join("nene_sample_arm.gltf");
    std::fs::write(&path, json).unwrap();
    path
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 0x3f) as usize] as char);
        out.push(T[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}
