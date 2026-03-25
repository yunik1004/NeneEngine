/// 3D physics: a cube falls and bounces off a flat floor (perspective view).
use nene::{
    math::{Mat4, Vec3},
    mesh::Model,
    physics3d::{ColliderBuilder, RigidBodyBuilder, RigidBodyHandle, World},
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
    let light   = normalize(vec3<f32>(1.0, 2.0, 3.0));
    let diffuse = max(dot(normalize(in.normal), light), 0.0);
    let color   = vec3<f32>(0.6, 0.75, 1.0) * (0.2 + 0.8 * diffuse);
    return vec4<f32>(color, 1.0);
}
"#;

const FLOOR_SHADER: &str = r#"
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

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_proj * camera.model * vec4<f32>(in.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.4, 0.4, 0.4, 1.0);
}
"#;

// Unit cube OBJ
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
    ball_pipeline: Pipeline,
    floor_pipeline: Pipeline,
    ball_vb: VertexBuffer,
    ball_ib: IndexBuffer,
    floor_vb: VertexBuffer,
    floor_ib: IndexBuffer,
    ball_uniform: UniformBuffer,
    floor_uniform: UniformBuffer,
    world: World,
    ball_handle: RigidBodyHandle,
}

fn write_temp_obj(name: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn scale_translate(sx: f32, sy: f32, sz: f32, tx: f32, ty: f32, tz: f32) -> [[f32; 4]; 4] {
    (Mat4::from_translation(Vec3::new(tx, ty, tz)) * Mat4::from_scale(Vec3::new(sx, sy, sz)))
        .to_cols_array_2d()
}

fn build_view_proj(aspect: f32) -> [[f32; 4]; 4] {
    let proj = Mat4::perspective_rh(45f32.to_radians(), aspect, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(4.0, 8.0, 12.0), Vec3::ZERO, Vec3::Y);
    (proj * view).to_cols_array_2d()
}

fn init(ctx: &mut Context) -> State {
    let mut world = World::new(); // gravity (0, -9.81, 0)

    // Fixed floor at y = -0.1 with a thick cuboid collider
    let floor_body = RigidBodyBuilder::fixed()
        .translation(0.0, -0.1, 0.0)
        .build();
    let floor_handle = world.add_body(floor_body);
    world.add_collider(ColliderBuilder::cuboid(5.0, 0.1, 5.0).build(), floor_handle);

    // Dynamic ball cube starting at y = 8
    let ball_body = RigidBodyBuilder::dynamic()
        .translation(0.0, 8.0, 0.0)
        .build();
    let ball_handle = world.add_body(ball_body);
    world.add_collider(
        ColliderBuilder::cuboid(0.5, 0.5, 0.5)
            .restitution(0.6)
            .build(),
        ball_handle,
    );

    let path = write_temp_obj("nene_physics3d_cube.obj", CUBE_OBJ);
    let model = Model::load(&path);
    let mesh = &model.meshes[0];

    let ball_vb = ctx.create_vertex_buffer(&mesh.vertices);
    let ball_ib = ctx.create_index_buffer(&mesh.indices);

    // Floor mesh: same cube, scaled to 10 x 0.2 x 10
    let floor_vb = ctx.create_vertex_buffer(&mesh.vertices);
    let floor_ib = ctx.create_index_buffer(&mesh.indices);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let view_proj = build_view_proj(aspect);

    let ball_uniform = ctx.create_uniform_buffer(&CameraUniform {
        view_proj,
        model: scale_translate(1.0, 1.0, 1.0, 0.0, 8.0, 0.0),
    });
    let floor_uniform = ctx.create_uniform_buffer(&CameraUniform {
        view_proj,
        model: scale_translate(10.0, 0.2, 10.0, 0.0, -0.1, 0.0),
    });

    let ball_pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, nene::mesh::MeshVertex::layout()).with_uniform(),
    );
    let floor_pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(FLOOR_SHADER, nene::mesh::MeshVertex::layout()).with_uniform(),
    );

    State {
        ball_pipeline,
        floor_pipeline,
        ball_vb,
        ball_ib,
        floor_vb,
        floor_ib,
        ball_uniform,
        floor_uniform,
        world,
        ball_handle,
    }
}

fn main() {
    Window::new(Config {
        title: "Physics 3D".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx| {
            state.world.step();

            let body = state.world.body(state.ball_handle).unwrap();
            let t = body.translation();
            let model = scale_translate(1.0, 1.0, 1.0, t.x, t.y, t.z);

            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            let view_proj = build_view_proj(aspect);

            ctx.update_uniform_buffer(&state.ball_uniform, &CameraUniform { view_proj, model });
            ctx.update_uniform_buffer(
                &state.floor_uniform,
                &CameraUniform {
                    view_proj,
                    model: scale_translate(10.0, 0.2, 10.0, 0.0, -0.1, 0.0),
                },
            );
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.floor_pipeline);
            pass.set_uniform(0, &state.floor_uniform);
            pass.set_vertex_buffer(0, &state.floor_vb);
            pass.draw_indexed(&state.floor_ib);

            pass.set_pipeline(&state.ball_pipeline);
            pass.set_uniform(0, &state.ball_uniform);
            pass.set_vertex_buffer(0, &state.ball_vb);
            pass.draw_indexed(&state.ball_ib);
        },
    );
}
