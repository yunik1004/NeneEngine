/// Load and render a glTF model with diffuse lighting and shadow mapping.
///
/// Usage: cargo run --example gltf -- path/to/model.gltf|glb
/// (argument optional — defaults to a built-in cube)
use nene::{
    camera::Camera,
    light::{AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight},
    math::{Mat4, Vec3},
    mesh::{MeshVertex, Model},
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    renderer::{FilterMode, SHADOW_WGSL, ShadowMap, Texture},
    uniform,
    window::{Config, Window},
};

const SHADOW_SHADER: &str = r#"
struct ShadowUniform {
    light_vp: mat4x4<f32>,
    model:    mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> u: ShadowUniform;

@vertex
fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.light_vp * u.model * vec4<f32>(pos, 1.0);
}
"#;

fn make_main_shader() -> String {
    format!(
        r#"
{SHADOW_WGSL}
{AMBIENT_LIGHT_WGSL}
{DIRECTIONAL_LIGHT_WGSL}

struct SceneUniform {{
    view_proj:   mat4x4<f32>,
    model:       mat4x4<f32>,
    light_vp:    mat4x4<f32>,
    ambient:     AmbientLight,
    directional: DirectionalLight,
}};
@group(0) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(0) var tex:  texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(2) @binding(0) var shadow_map:  texture_depth_2d;
@group(2) @binding(1) var shadow_samp: sampler_comparison;

struct VertexInput {{
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}};
struct VertexOutput {{
    @builtin(position) clip_pos:     vec4<f32>,
    @location(0)       world_normal: vec3<f32>,
    @location(1)       uv:           vec2<f32>,
    @location(2)       light_space:  vec4<f32>,
}};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    let world_pos = scene.model * vec4<f32>(in.position, 1.0);
    var out: VertexOutput;
    out.clip_pos     = scene.view_proj * world_pos;
    out.world_normal = (scene.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv           = in.uv;
    out.light_space  = scene.light_vp * world_pos;
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let albedo  = textureSample(tex, samp, in.uv);
    let shadow  = shadow_factor(shadow_map, shadow_samp, in.light_space, 0.0);
    let diffuse = directional_light(scene.directional, in.world_normal) * shadow;
    let ambient = ambient_light(scene.ambient);
    return vec4<f32>(albedo.rgb * (ambient + diffuse), albedo.a);
}}
"#,
        SHADOW_WGSL = SHADOW_WGSL,
        AMBIENT_LIGHT_WGSL = AMBIENT_LIGHT_WGSL,
        DIRECTIONAL_LIGHT_WGSL = DIRECTIONAL_LIGHT_WGSL,
    )
}

#[uniform]
struct SceneUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_vp: [[f32; 4]; 4],
    ambient: AmbientLight,
    directional: DirectionalLight,
}

#[uniform]
struct ShadowUniform {
    light_vp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

fn build_scene(
    model: Mat4,
    aspect: f32,
    ambient: &AmbientLight,
    directional: &DirectionalLight,
) -> SceneUniform {
    let camera = Camera::perspective(Vec3::new(0.0, 2.0, 6.0), 45.0, 0.1, 100.0);
    let light_vp = directional.light_view_proj(Vec3::ZERO, 5.0);
    SceneUniform {
        view_proj: camera.view_proj(aspect).to_cols_array_2d(),
        model: model.to_cols_array_2d(),
        light_vp: light_vp.to_cols_array_2d(),
        ambient: *ambient,
        directional: *directional,
    }
}

fn build_shadow(model: Mat4, directional: &DirectionalLight) -> ShadowUniform {
    let light_vp = directional.light_view_proj(Vec3::ZERO, 5.0);
    ShadowUniform {
        light_vp: light_vp.to_cols_array_2d(),
        model: model.to_cols_array_2d(),
    }
}

struct MeshGpu {
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    texture: Texture,
    transform: Mat4,
}

struct State {
    shadow_pipeline: Pipeline,
    main_pipeline: Pipeline,
    meshes: Vec<MeshGpu>,
    scene_buffers: Vec<UniformBuffer>,
    shadow_buffers: Vec<UniformBuffer>,
    shadow_map: ShadowMap,
    ambient: AmbientLight,
    directional: DirectionalLight,
    angle: f32,
}

fn white_texture(ctx: &mut Context) -> Texture {
    ctx.create_texture_with(1, 1, &[255, 255, 255, 255], FilterMode::Nearest)
}

fn init(ctx: &mut Context, path: &str) -> State {
    let model = Model::load(path);

    let ambient = AmbientLight::new(Vec3::ONE, 0.15);
    let directional =
        DirectionalLight::new(Vec3::new(1.0, -2.0, -1.0), Vec3::new(1.0, 0.95, 0.9), 1.0);

    let shadow_map = ctx.create_shadow_map(1024);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;

    let mut meshes = Vec::new();
    let mut scene_buffers = Vec::new();
    let mut shadow_buffers = Vec::new();

    for mesh in &model.meshes {
        let vertex_buffer = ctx.create_vertex_buffer(&mesh.vertices);
        let index_buffer = ctx.create_index_buffer(&mesh.indices);
        let texture = match &mesh.base_color {
            Some(img) => {
                ctx.create_texture_with(img.width, img.height, &img.data, FilterMode::Linear)
            }
            None => white_texture(ctx),
        };
        scene_buffers.push(ctx.create_uniform_buffer(&build_scene(
            mesh.transform,
            aspect,
            &ambient,
            &directional,
        )));
        shadow_buffers.push(ctx.create_uniform_buffer(&build_shadow(mesh.transform, &directional)));
        meshes.push(MeshGpu {
            vertex_buffer,
            index_buffer,
            texture,
            transform: mesh.transform,
        });
    }

    let shadow_pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADOW_SHADER, MeshVertex::layout())
            .with_uniform()
            .depth_only(),
    );

    let main_shader = make_main_shader();
    let main_pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(&main_shader, MeshVertex::layout())
            .with_uniform()
            .with_texture()
            .with_shadow_map()
            .with_depth(),
    );

    State {
        shadow_pipeline,
        main_pipeline,
        meshes,
        scene_buffers,
        shadow_buffers,
        shadow_map,
        ambient,
        directional,
        angle: 0.0,
    }
}

/// Write a minimal cube glTF (with positions, normals, uvs) to a temp file and return its path.
fn write_sample_gltf() -> std::path::PathBuf {
    // 24 vertices: position[3] + normal[3] + uv[2]
    let verts: &[[[f32; 3]; 2]; 24] = &[
        // +Z
        [[-0.5, -0.5, 0.5], [0., 0., 1.]],
        [[0.5, -0.5, 0.5], [0., 0., 1.]],
        [[0.5, 0.5, 0.5], [0., 0., 1.]],
        [[-0.5, 0.5, 0.5], [0., 0., 1.]],
        // -Z
        [[0.5, -0.5, -0.5], [0., 0., -1.]],
        [[-0.5, -0.5, -0.5], [0., 0., -1.]],
        [[-0.5, 0.5, -0.5], [0., 0., -1.]],
        [[0.5, 0.5, -0.5], [0., 0., -1.]],
        // +Y
        [[-0.5, 0.5, 0.5], [0., 1., 0.]],
        [[0.5, 0.5, 0.5], [0., 1., 0.]],
        [[0.5, 0.5, -0.5], [0., 1., 0.]],
        [[-0.5, 0.5, -0.5], [0., 1., 0.]],
        // -Y
        [[-0.5, -0.5, -0.5], [0., -1., 0.]],
        [[0.5, -0.5, -0.5], [0., -1., 0.]],
        [[0.5, -0.5, 0.5], [0., -1., 0.]],
        [[-0.5, -0.5, 0.5], [0., -1., 0.]],
        // +X
        [[0.5, -0.5, 0.5], [1., 0., 0.]],
        [[0.5, -0.5, -0.5], [1., 0., 0.]],
        [[0.5, 0.5, -0.5], [1., 0., 0.]],
        [[0.5, 0.5, 0.5], [1., 0., 0.]],
        // -X
        [[-0.5, -0.5, -0.5], [-1., 0., 0.]],
        [[-0.5, -0.5, 0.5], [-1., 0., 0.]],
        [[-0.5, 0.5, 0.5], [-1., 0., 0.]],
        [[-0.5, 0.5, -0.5], [-1., 0., 0.]],
    ];
    let uvs: &[[f32; 2]; 24] = &[
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
    ];
    let indices: Vec<u32> = (0..6u32)
        .flat_map(|f| {
            let b = f * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .collect();

    let mut buf: Vec<u8> = Vec::new();
    // accessor 0: positions (VEC3 float)
    let pos_off = buf.len();
    for v in verts {
        for &f in &v[0] {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let pos_len = buf.len() - pos_off;
    // accessor 1: normals (VEC3 float)
    let nor_off = buf.len();
    for v in verts {
        for &f in &v[1] {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let nor_len = buf.len() - nor_off;
    // accessor 2: uvs (VEC2 float)
    let uv_off = buf.len();
    for uv in uvs {
        for &f in uv {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let uv_len = buf.len() - uv_off;
    // accessor 3: indices (SCALAR u32)
    let idx_off = buf.len();
    for &i in &indices {
        buf.extend_from_slice(&i.to_le_bytes());
    }
    let idx_len = buf.len() - idx_off;

    let b64 = base64_encode(&buf);
    let total = buf.len();
    let json = format!(
        r#"{{
  "asset": {{"version":"2.0"}},
  "scene": 0,
  "scenes": [{{"nodes":[0]}}],
  "nodes": [{{"mesh":0}}],
  "meshes": [{{"primitives":[{{"attributes":{{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2}},"indices":3}}]}}],
  "accessors": [
    {{"bufferView":0,"componentType":5126,"count":24,"type":"VEC3","min":[-0.5,-0.5,-0.5],"max":[0.5,0.5,0.5]}},
    {{"bufferView":1,"componentType":5126,"count":24,"type":"VEC3"}},
    {{"bufferView":2,"componentType":5126,"count":24,"type":"VEC2"}},
    {{"bufferView":3,"componentType":5125,"count":36,"type":"SCALAR"}}
  ],
  "bufferViews": [
    {{"buffer":0,"byteOffset":{pos_off},"byteLength":{pos_len}}},
    {{"buffer":0,"byteOffset":{nor_off},"byteLength":{nor_len}}},
    {{"buffer":0,"byteOffset":{uv_off}, "byteLength":{uv_len}}},
    {{"buffer":0,"byteOffset":{idx_off},"byteLength":{idx_len}}}
  ],
  "buffers": [{{"byteLength":{total},"uri":"data:application/octet-stream;base64,{b64}"}}]
}}"#
    );
    let path = std::env::temp_dir().join("nene_sample_cube.gltf");
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

fn main() {
    let tmp;
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            tmp = write_sample_gltf();
            tmp.to_str().unwrap().to_string()
        }
    };
    Window::new(Config {
        title: "glTF".to_string(),
        ..Config::default()
    })
    .run_with_update(
        move |ctx| init(ctx, &path),
        |state, ctx, _input, time| {
            state.angle += std::f32::consts::TAU * 0.1 * time.delta;
            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            let rotation = Mat4::from_rotation_y(state.angle);
            for (i, mesh) in state.meshes.iter().enumerate() {
                let model = rotation * mesh.transform;
                ctx.update_uniform_buffer(
                    &state.scene_buffers[i],
                    &build_scene(model, aspect, &state.ambient, &state.directional),
                );
                ctx.update_uniform_buffer(
                    &state.shadow_buffers[i],
                    &build_shadow(model, &state.directional),
                );
            }
        },
        |state, ctx| {
            ctx.shadow_pass(&state.shadow_map, |pass| {
                pass.set_pipeline(&state.shadow_pipeline);
                for (i, mesh) in state.meshes.iter().enumerate() {
                    pass.set_uniform(0, &state.shadow_buffers[i]);
                    pass.set_vertex_buffer(0, &mesh.vertex_buffer);
                    pass.draw_indexed(&mesh.index_buffer);
                }
            });
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.main_pipeline);
            for (i, mesh) in state.meshes.iter().enumerate() {
                pass.set_uniform(0, &state.scene_buffers[i]);
                pass.set_texture(1, &mesh.texture);
                pass.set_shadow_map(2, &state.shadow_map);
                pass.set_vertex_buffer(0, &mesh.vertex_buffer);
                pass.draw_indexed(&mesh.index_buffer);
            }
        },
    );
}
