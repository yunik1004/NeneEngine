use nene::{
    camera::Camera,
    math::{Mat4, Vec3},
    mesh::Model,
    renderer::{
        Context, FilterMode, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, Texture,
        UniformBuffer, VertexBuffer,
    },
    uniform,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    model:     mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var tex:     texture_2d<f32>;
@group(1) @binding(1) var samp:    sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv:     vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = camera.view_proj * camera.model * vec4<f32>(in.position, 1.0);
    out.normal   = in.normal;
    out.uv       = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light   = normalize(vec3<f32>(1.0, 2.0, 3.0));
    let diffuse = max(dot(normalize(in.normal), light), 0.0);
    let albedo  = textureSample(tex, samp, in.uv);
    return vec4<f32>(albedo.rgb * (0.2 + 0.8 * diffuse), albedo.a);
}
"#;

// Quad with normals and UVs
const QUAD_OBJ: &str = "\
v -0.5 -0.5 0.0
v  0.5 -0.5 0.0
v  0.5  0.5 0.0
v -0.5  0.5 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
";

#[uniform]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

struct State {
    pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    camera_buffer: UniformBuffer,
    texture: Texture,
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
                (220, 200, 255)
            } else {
                (80, 40, 120)
            };
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    ctx.create_texture_with(size, size, &data, FilterMode::Nearest)
}

fn write_quad_obj() -> std::path::PathBuf {
    let path = std::env::temp_dir().join("nene_textured_quad.obj");
    std::fs::write(&path, QUAD_OBJ).unwrap();
    path
}

fn build_camera(angle: f32, aspect: f32) -> CameraUniform {
    let camera = Camera::perspective(Vec3::new(0.0, 0.0, 3.0), 45.0, 0.1, 100.0);
    CameraUniform {
        view_proj: camera.view_proj(aspect).to_cols_array_2d(),
        model: Mat4::from_rotation_y(angle).to_cols_array_2d(),
    }
}

fn init(ctx: &mut Context) -> State {
    let path = write_quad_obj();
    let model = Model::load(&path);
    let mesh = &model.meshes[0];

    let vertex_buffer = ctx.create_vertex_buffer(&mesh.vertices);
    let index_buffer = ctx.create_index_buffer(&mesh.indices);
    let texture = make_checkerboard(ctx);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let camera_buffer = ctx.create_uniform_buffer(&build_camera(0.0, aspect));

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, nene::mesh::MeshVertex::layout())
            .with_uniform()
            .with_texture(),
    );

    State {
        pipeline,
        vertex_buffer,
        index_buffer,
        camera_buffer,
        texture,
        angle: 0.0,
    }
}

fn main() {
    Window::new(Config {
        title: "Textured Mesh".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, _input, time| {
            state.angle += std::f32::consts::TAU * 0.1 * time.delta;
            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            ctx.update_uniform_buffer(&state.camera_buffer, &build_camera(state.angle, aspect));
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.camera_buffer);
            pass.set_texture(1, &state.texture);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw_indexed(&state.index_buffer);
        },
    );
}
