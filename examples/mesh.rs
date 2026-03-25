use nene::{
    mesh::Model,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = camera.view_proj * camera.model * vec4<f32>(in.position, 1.0);
    out.normal   = in.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(1.0, 2.0, 3.0));
    let diffuse = max(dot(normalize(in.normal), light), 0.0);
    let color = vec3<f32>(0.6, 0.75, 1.0) * (0.2 + 0.8 * diffuse);
    return vec4<f32>(color, 1.0);
}
"#;

// Unit cube OBJ (no index reuse so tobj single_index works cleanly)
const CUBE_OBJ: &str = "\
v -0.5 -0.5  0.5
v  0.5 -0.5  0.5
v  0.5  0.5  0.5
v -0.5  0.5  0.5
v -0.5 -0.5 -0.5
v  0.5 -0.5 -0.5
v  0.5  0.5 -0.5
v -0.5  0.5 -0.5
vn  0  0  1
vn  0  0 -1
vn  0  1  0
vn  0 -1  0
vn  1  0  0
vn -1  0  0
f 1//1 2//1 3//1
f 1//1 3//1 4//1
f 6//2 5//2 8//2
f 6//2 8//2 7//2
f 4//3 3//3 7//3
f 4//3 7//3 8//3
f 5//4 6//4 2//4
f 5//4 2//4 1//4
f 2//5 6//5 7//5
f 2//5 7//5 3//5
f 5//6 1//6 4//6
f 5//6 4//6 8//6
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
    angle: f32,
}

fn write_temp_obj() -> std::path::PathBuf {
    let path = std::env::temp_dir().join("nene_mesh_cube.obj");
    std::fs::write(&path, CUBE_OBJ).unwrap();
    path
}

fn build_camera(angle: f32, aspect: f32) -> CameraUniform {
    use nene::math::{Mat4, Vec3};
    let proj = Mat4::perspective_rh(45f32.to_radians(), aspect, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 1.5, 4.0), Vec3::ZERO, Vec3::Y);
    let model = Mat4::from_rotation_y(angle);
    CameraUniform {
        view_proj: (proj * view).to_cols_array_2d(),
        model: model.to_cols_array_2d(),
    }
}

fn init(ctx: &mut Context) -> State {
    let path = write_temp_obj();
    let model = Model::load(&path);
    let mesh = &model.meshes[0];

    let vertex_buffer = ctx.create_vertex_buffer(&mesh.vertices);
    let index_buffer = ctx.create_index_buffer(&mesh.indices);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let camera = build_camera(0.0, aspect);
    let camera_buffer = ctx.create_uniform_buffer(&camera);

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, nene::mesh::MeshVertex::layout()).with_uniform(),
    );

    State {
        pipeline,
        vertex_buffer,
        index_buffer,
        camera_buffer,
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
        |state, ctx, _input| {
            state.angle += 0.01;
            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            let camera = build_camera(state.angle, aspect);
            ctx.update_uniform_buffer(&state.camera_buffer, &camera);
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.camera_buffer);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw_indexed(&state.index_buffer);
        },
    );
}
