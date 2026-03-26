/// Spinning triangle rendered through the post-process stack.
/// Press keys to toggle effects:
///   1 - cycle tone mapping (None / Reinhard / ACES)
///   2 - toggle vignette
///   3 - toggle desaturation
use nene::{
    input::Key,
    renderer::postprocess::{PostProcessStack, ToneMap},
    renderer::{Context, Pipeline, PipelineDescriptor, RenderPass, VertexBuffer},
    time::Time,
    vertex,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct Uniform {
    angle: f32,
};
@group(0) @binding(0) var<uniform> u: Uniform;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color:    vec3<f32>,
) -> VSOut {
    let s = sin(u.angle);
    let c = cos(u.angle);
    let rot = vec2<f32>(
        position.x * c - position.y * s,
        position.x * s + position.y * c,
    );
    return VSOut(vec4<f32>(rot, 0.0, 1.0), color);
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

#[vertex]
struct Vert {
    position: [f32; 2],
    color: [f32; 3],
}

#[nene::uniform]
struct TriUniform {
    angle: f32,
}

struct State {
    pp: PostProcessStack,
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ubuf: nene::renderer::UniformBuffer,
    angle: f32,
}

fn init(ctx: &mut Context) -> State {
    let cfg = ctx.surface_config();
    let (w, h) = (cfg.width, cfg.height);

    let pp = PostProcessStack::new(ctx, w, h);

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, Vert::layout())
            .with_uniform()
            .with_depth(),
    );

    let vertices = &[
        Vert {
            position: [0.0, 0.6],
            color: [1.0, 0.2, 0.2],
        },
        Vert {
            position: [-0.5, -0.4],
            color: [0.2, 1.0, 0.2],
        },
        Vert {
            position: [0.5, -0.4],
            color: [0.2, 0.2, 1.0],
        },
    ];
    let vbuf = ctx.create_vertex_buffer(vertices);
    let ubuf = ctx.create_uniform_buffer(&TriUniform { angle: 0.0 });

    State {
        pp,
        pipeline,
        vbuf,
        ubuf,
        angle: 0.0,
    }
}

fn main() {
    Window::new(Config {
        title: "Post-Process".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time: &Time| {
            state.angle += std::f32::consts::TAU * 0.25 * time.delta;

            let mut dirty = false;
            // 1 — cycle tone mapping
            if input.key_pressed(Key::Digit1) {
                state.pp.settings.tone_map = match state.pp.settings.tone_map {
                    ToneMap::None => ToneMap::Reinhard,
                    ToneMap::Reinhard => ToneMap::Aces,
                    ToneMap::Aces => ToneMap::None,
                };
                dirty = true;
            }
            // 2 — toggle vignette
            if input.key_pressed(Key::Digit2) {
                state.pp.settings.vignette = if state.pp.settings.vignette > 0.0 {
                    0.0
                } else {
                    1.2
                };
                dirty = true;
            }
            // 3 — toggle desaturation
            if input.key_pressed(Key::Digit3) {
                state.pp.settings.saturation = if state.pp.settings.saturation > 0.5 {
                    0.0
                } else {
                    1.0
                };
                dirty = true;
            }
            if dirty {
                state.pp.apply_settings(ctx);
            }

            ctx.update_uniform_buffer(&state.ubuf, &TriUniform { angle: state.angle });
        },
        |state, ctx| {
            state.pp.scene_pass(ctx, |pass| {
                pass.set_pipeline(&state.pipeline);
                pass.set_uniform(0, &state.ubuf);
                pass.set_vertex_buffer(0, &state.vbuf);
                pass.draw(0..3);
            });
        },
        |state, pass: &mut RenderPass| {
            state.pp.apply(pass);
        },
    );
}
