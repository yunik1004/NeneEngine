use nene::{
    renderer::{
        Context, FilterMode, Pipeline, PipelineDescriptor, RenderPass, Texture, VertexBuffer,
    },
    vertex,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t, s, in.uv);
}
"#;

#[vertex]
struct QuadVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

// 사각형 = 삼각형 2개
const VERTICES: &[QuadVertex] = &[
    QuadVertex {
        position: [-0.5, -0.5],
        uv: [0.0, 1.0],
    },
    QuadVertex {
        position: [0.5, -0.5],
        uv: [1.0, 1.0],
    },
    QuadVertex {
        position: [-0.5, 0.5],
        uv: [0.0, 0.0],
    },
    QuadVertex {
        position: [-0.5, 0.5],
        uv: [0.0, 0.0],
    },
    QuadVertex {
        position: [0.5, -0.5],
        uv: [1.0, 1.0],
    },
    QuadVertex {
        position: [0.5, 0.5],
        uv: [1.0, 0.0],
    },
];

struct State {
    pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
    texture: Texture,
}

fn make_checkerboard(ctx: &mut Context) -> Texture {
    let size = 64u32;
    let tile = 8u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let white = ((x / tile) + (y / tile)) % 2 == 0;
            let c = if white { 240 } else { 80 };
            data.extend_from_slice(&[c, c, c, 255]);
        }
    }
    ctx.create_texture_with(size, size, &data, FilterMode::Nearest)
}

fn main() {
    Window::new(Config {
        title: "Texture".to_string(),
        ..Config::default()
    })
    .run_with(
        |ctx| State {
            texture: make_checkerboard(ctx),
            vertex_buffer: ctx.create_vertex_buffer(VERTICES),
            pipeline: ctx.create_pipeline(
                PipelineDescriptor::new(SHADER, QuadVertex::layout()).with_texture(),
            ),
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_texture(0, &state.texture);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw(0..6);
        },
    );
}
