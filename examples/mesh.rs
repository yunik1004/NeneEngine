/// Rotating textured cube with diffuse lighting and shadow mapping.
use nene::{
    camera::Camera,
    light::{AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight},
    math::{Mat4, Vec3},
    mesh::MeshVertex,
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
    view_proj: mat4x4<f32>,
    model:     mat4x4<f32>,
    light_vp:  mat4x4<f32>,
    ambient:   AmbientLight,
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
    out.clip_pos    = scene.view_proj * world_pos;
    out.world_normal = (scene.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv          = in.uv;
    out.light_space = scene.light_vp * world_pos;
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
        v!(-0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 1.0),
        v!(0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0),
        v!(0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0),
        v!(-0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0),
        v!(0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 1.0),
        v!(-0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 1.0),
        v!(-0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 0.0),
        v!(0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 0.0),
        v!(-0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 1.0),
        v!(0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 1.0, 1.0),
        v!(0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 1.0, 0.0),
        v!(-0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 0.0),
        v!(-0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.0, 1.0),
        v!(0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 1.0, 1.0),
        v!(0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 1.0, 0.0),
        v!(-0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 0.0, 0.0),
        v!(0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 1.0),
        v!(0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 1.0),
        v!(0.5, 0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 0.0),
        v!(0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0),
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

struct State {
    shadow_pipeline: Pipeline,
    main_pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    scene_buffer: UniformBuffer,
    shadow_buffer: UniformBuffer,
    texture: Texture,
    shadow_map: ShadowMap,
    ambient: AmbientLight,
    directional: DirectionalLight,
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

fn build_scene(
    angle: f32,
    aspect: f32,
    ambient: &AmbientLight,
    directional: &DirectionalLight,
) -> SceneUniform {
    let camera = Camera::perspective(Vec3::new(0.0, 1.5, 4.0), 45.0, 0.1, 100.0);
    let light_vp = directional.light_view_proj(Vec3::ZERO, 3.0);
    SceneUniform {
        view_proj: camera.view_proj(aspect).to_cols_array_2d(),
        model: Mat4::from_rotation_y(angle).to_cols_array_2d(),
        light_vp: light_vp.to_cols_array_2d(),
        ambient: *ambient,
        directional: *directional,
    }
}

fn build_shadow(angle: f32, directional: &DirectionalLight) -> ShadowUniform {
    let light_vp = directional.light_view_proj(Vec3::ZERO, 3.0);
    ShadowUniform {
        light_vp: light_vp.to_cols_array_2d(),
        model: Mat4::from_rotation_y(angle).to_cols_array_2d(),
    }
}

fn init(ctx: &mut Context) -> State {
    let (vertices, indices) = cube_mesh();
    let vertex_buffer = ctx.create_vertex_buffer(&vertices);
    let index_buffer = ctx.create_index_buffer(&indices);
    let texture = make_checkerboard(ctx);
    let shadow_map = ctx.create_shadow_map(1024);

    let ambient = AmbientLight::new(Vec3::ONE, 0.15);
    let directional =
        DirectionalLight::new(Vec3::new(1.0, -2.0, -1.0), Vec3::new(1.0, 0.95, 0.9), 1.0);

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let scene_buffer = ctx.create_uniform_buffer(&build_scene(0.0, aspect, &ambient, &directional));
    let shadow_buffer = ctx.create_uniform_buffer(&build_shadow(0.0, &directional));

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
        vertex_buffer,
        index_buffer,
        scene_buffer,
        shadow_buffer,
        texture,
        shadow_map,
        ambient,
        directional,
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
                &build_scene(state.angle, aspect, &state.ambient, &state.directional),
            );
            ctx.update_uniform_buffer(
                &state.shadow_buffer,
                &build_shadow(state.angle, &state.directional),
            );
        },
        |state, ctx| {
            ctx.shadow_pass(&state.shadow_map, |pass| {
                pass.set_pipeline(&state.shadow_pipeline);
                pass.set_uniform(0, &state.shadow_buffer);
                pass.set_vertex_buffer(0, &state.vertex_buffer);
                pass.draw_indexed(&state.index_buffer);
            });
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.main_pipeline);
            pass.set_uniform(0, &state.scene_buffer);
            pass.set_texture(1, &state.texture);
            pass.set_shadow_map(2, &state.shadow_map);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw_indexed(&state.index_buffer);
        },
    );
}
