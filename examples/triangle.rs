use nene::{
    renderer::{Pipeline, PipelineDescriptor, RenderPass, VertexBuffer},
    vertex,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

#[vertex]
struct TriangleVertex {
    position: [f32; 2],
    color: [f32; 3],
}

const VERTICES: &[TriangleVertex] = &[
    TriangleVertex {
        position: [0.0, 0.5],
        color: [1.0, 0.0, 0.0],
    },
    TriangleVertex {
        position: [-0.5, -0.5],
        color: [0.0, 1.0, 0.0],
    },
    TriangleVertex {
        position: [0.5, -0.5],
        color: [0.0, 0.0, 1.0],
    },
];

struct State {
    pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
}

fn main() {
    Window::new(Config {
        title: "Triangle".to_string(),
        ..Config::default()
    })
    .run_with(
        |ctx| State {
            vertex_buffer: ctx.create_vertex_buffer(VERTICES),
            pipeline: ctx
                .create_pipeline(PipelineDescriptor::new(SHADER, TriangleVertex::layout())),
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_vertex_buffer(0, &state.vertex_buffer);
            pass.draw(0..3);
        },
    );
}
