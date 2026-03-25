/// Rotating textured cube with diffuse lighting.
use nene::{
    camera::Camera,
    light::{DIRECTIONAL_LIGHT_WGSL, DirectionalLight, DirectionalLightUniform},
    math::{Mat4, Vec3},
    mesh::MeshVertex,
    renderer::{
        Context, FilterMode, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, Texture,
        UniformBuffer, VertexBuffer,
    },
    window::{Config, Window},
};

// Shader is assembled from the reusable light snippet + scene-specific code.
fn make_shader() -> String {
    format!(
        r#"
{DIRECTIONAL_LIGHT_WGSL}

struct SceneUniform {{
    view_proj: mat4x4<f32>,
    model:     mat4x4<f32>,
    light:     DirectionalLight,
}};
@group(0) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(0) var tex:  texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VertexInput {{
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}};
struct VertexOutput {{
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv:           vec2<f32>,
}};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    out.clip_pos    = scene.view_proj * scene.model * vec4<f32>(in.position, 1.0);
    out.world_normal = in.normal;
    out.uv           = in.uv;
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let albedo  = textureSample(tex, samp, in.uv);
    let diffuse = directional_light(scene.light, in.world_normal);
    let ambient = scene.light.color * 0.15;
    return vec4<f32>(albedo.rgb * (ambient + diffuse), albedo.a);
}}
"#,
        DIRECTIONAL_LIGHT_WGSL = DIRECTIONAL_LIGHT_WGSL,
    )
}

// ── GPU uniform (camera + light packed together) ───────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SceneUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light: DirectionalLightUniform,
}

// ── Cube geometry ─────────────────────────────────────────────────────────────

fn cube_mesh() -> (Vec<MeshVertex>, Vec<u32>) {
    macro_rules! v {
        ($px:expr, $py:expr, $pz:expr, $nx:expr, $ny:expr, $nz:expr, $u:expr, $vv:expr) => {
            MeshVertex {
                position: [$px, $py, $pz],
                normal: [$nx, $ny, $nz],
                uv: [$u, $vv],
            }
        };
    }
    let vertices = vec![
        // +Z front
        v!(-0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 1.0),
        v!(0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0),
        v!(0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0),
        v!(-0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0),
        // -Z back
        v!(0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 1.0),
        v!(-0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 1.0),
        v!(-0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 0.0),
        v!(0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 0.0),
        // +Y top
        v!(-0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 1.0),
        v!(0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 1.0, 1.0),
        v!(0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 1.0, 0.0),
        v!(-0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 0.0),
        // -Y bottom
        v!(-0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.0, 1.0),
        v!(0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 1.0, 1.0),
        v!(0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 1.0, 0.0),
        v!(-0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 0.0, 0.0),
        // +X right
        v!(0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 1.0),
        v!(0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 1.0),
        v!(0.5, 0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 0.0),
        v!(0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0),
        // -X left
        v!(-0.5, -0.5, -0.5, -1.0, 0.0, 0.0, 0.0, 1.0),
        v!(-0.5, -0.5, 0.5, -1.0, 0.0, 0.0, 1.0, 1.0),
        v!(-0.5, 0.5, 0.5, -1.0, 0.0, 0.0, 1.0, 0.0),
        v!(-0.5, 0.5, -0.5, -1.0, 0.0, 0.0, 0.0, 0.0),
    ];
    let indices: Vec<u32> = (0..6u32)
        .flat_map(|f| {
            let b = f * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .collect();
    (vertices, indices)
}

// ── App ───────────────────────────────────────────────────────────────────────

struct State {
    pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    scene_buffer: UniformBuffer,
    texture: Texture,
    light: DirectionalLight,
    angle: f32,
}

fn make_checkerboard(ctx: &mut Context) -> Texture {
    let size = 128u32;
    let tile = 16u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let white = ((x / tile) + (y / tile)) % 2 == 0;
            let (r, g, b) = if white {
                (220u8, 200, 255)
            } else {
                (80u8, 40, 120)
            };
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    ctx.create_texture_with(size, size, &data, FilterMode::Nearest)
}

fn build_scene(angle: f32, aspect: f32, light: &DirectionalLight) -> SceneUniform {
    let camera = Camera::perspective(Vec3::new(0.0, 1.5, 4.0), 45.0, 0.1, 100.0);
    SceneUniform {
        view_proj: camera.view_proj(aspect).to_cols_array_2d(),
        model: Mat4::from_rotation_y(angle).to_cols_array_2d(),
        light: light.to_uniform(),
    }
}

fn init(ctx: &mut Context) -> State {
    let (vertices, indices) = cube_mesh();
    let vertex_buffer = ctx.create_vertex_buffer(&vertices);
    let index_buffer = ctx.create_index_buffer(&indices);
    let texture = make_checkerboard(ctx);
    let light = DirectionalLight::new(Vec3::new(1.0, -2.0, -1.0), [1.0, 0.95, 0.9], 1.0);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let scene_buffer = ctx.create_uniform_buffer(&build_scene(0.0, aspect, &light));

    let shader = make_shader();
    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(&shader, MeshVertex::layout())
            .with_uniform()
            .with_texture()
            .with_depth(),
    );

    State {
        pipeline,
        vertex_buffer,
        index_buffer,
        scene_buffer,
        texture,
        light,
        angle: 0.0,
    }
}

fn main() {
    Window::new(Config {
        title: "Mesh".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, _input, time| {
            state.angle += std::f32::consts::TAU * 0.1 * time.delta;
            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            ctx.update_uniform_buffer(
                &state.scene_buffer,
                &build_scene(state.angle, aspect, &state.light),
            );
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.scene_buffer);
            pass.set_texture(1, &state.texture);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw_indexed(&state.index_buffer);
        },
    );
}
