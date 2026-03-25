/// Render text into a texture each frame and display it on a rotating 3D quad.
use nene::{
    camera::Camera,
    math::{Mat4, Vec3},
    mesh::MeshVertex,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    text::TextRenderer,
    texture::Texture,
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
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
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
        view_proj: camera.view_proj(aspect).to_cols_array_2d(),
        model: Mat4::IDENTITY.to_cols_array_2d(),
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
        title: "Text → Texture".to_string(),
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
                    view_proj: camera.view_proj(aspect).to_cols_array_2d(),
                    model: Mat4::from_rotation_y(state.angle).to_cols_array_2d(),
                },
            );

            // Re-render text to texture every frame
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
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.scene_buffer);
            pass.set_texture(1, &state.label_texture);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw_indexed(&state.index_buffer);
        },
    );
}
