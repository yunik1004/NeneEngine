//! Text rendering demo.
//!
//! Shows two ways to use [`TextRenderer`]:
//!
//! 1. **Screen overlay** — `queue` + `render`: draw text directly onto the
//!    screen each frame, zero allocation at display time.
//! 2. **Texture bake** — `render_to_texture`: rasterise text into a [`Texture`]
//!    and sample it on a spinning 3D quad.
use nene::{
    camera::Camera,
    math::{Mat4, Vec3},
    mesh::MeshVertex,
    renderer::Texture,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    text::TextRenderer,
    uniform,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct SceneUniform {
    view_proj: mat4x4<f32>,
    model:     mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(0) var tex:  texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) _n: vec3<f32>, @location(2) uv: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = scene.view_proj * scene.model * vec4<f32>(pos, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

#[uniform]
struct SceneUniform {
    view_proj: Mat4,
    model: Mat4,
}

struct State {
    pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    scene_buffer: UniformBuffer,
    label_texture: Texture,
    text: TextRenderer,
    angle: f32,
    frame: u32,
}

fn quad_mesh() -> (Vec<MeshVertex>, Vec<u32>) {
    // A quad in the XY plane, sized to match the 4:1 texture aspect ratio
    let w = 2.0f32;
    let h = 0.5f32;
    let verts = vec![
        MeshVertex {
            position: [-w, -h, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [w, -h, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [w, h, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [-w, h, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    (verts, indices)
}

fn init(ctx: &mut Context) -> State {
    let (verts, indices) = quad_mesh();
    let vertex_buffer = ctx.create_vertex_buffer(&verts);
    let index_buffer = ctx.create_index_buffer(&indices);

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, MeshVertex::layout())
            .with_uniform()
            .with_texture()
            .with_depth()
            .with_alpha_blend(),
    );

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let camera = Camera::perspective(Vec3::new(0.0, 0.0, 4.0), 45.0, 0.1, 100.0);
    let scene_buffer = ctx.create_uniform_buffer(&SceneUniform {
        view_proj: camera.view_proj(aspect),
        model: Mat4::IDENTITY,
    });

    let mut text = TextRenderer::new(ctx);
    text.queue("Frame: 0", 10.0, 10.0, 48.0, [1.0, 1.0, 1.0, 1.0]);
    let label_texture = text.render_to_texture(ctx, 512, 128);

    State {
        pipeline,
        vertex_buffer,
        index_buffer,
        scene_buffer,
        label_texture,
        text,
        angle: 0.0,
        frame: 0,
    }
}

fn main() {
    Window::new(Config {
        title: "Text demo — overlay + texture bake",
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, _input, time| {
            state.angle += std::f32::consts::TAU * 0.3 * time.delta;
            state.frame += 1;

            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;
            let camera = Camera::perspective(Vec3::new(0.0, 0.0, 4.0), 45.0, 0.1, 100.0);
            ctx.update_uniform_buffer(
                &state.scene_buffer,
                &SceneUniform {
                    view_proj: camera.view_proj(aspect),
                    model: Mat4::from_rotation_y(state.angle),
                },
            );

            // Bake text into the 3-D quad texture
            state.text.queue(
                &format!("Frame: {}", state.frame),
                10.0,
                10.0,
                48.0,
                [1.0, 1.0, 1.0, 1.0],
            );
            state
                .text
                .queue("nene engine", 10.0, 70.0, 32.0, [0.6, 0.9, 1.0, 1.0]);
            state.label_texture = state.text.render_to_texture(ctx, 512, 128);

            // 2-D screen overlay (direct render, no texture)
            state
                .text
                .queue("Hello, Nene!", 20.0, 20.0, 36.0, [1.0, 1.0, 1.0, 1.0]);
            state.text.queue(
                "↑ text baked into texture  ↑",
                20.0,
                64.0,
                20.0,
                [0.6, 0.6, 0.6, 1.0],
            );
            state.text.prepare(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            // 3-D textured quad
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.scene_buffer);
            pass.set_texture(1, &state.label_texture);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw_indexed(&state.index_buffer);

            // 2-D screen overlay (direct text render)
            state.text.render(pass);
        },
    );
}
