/// Demonstrates the input module: keyboard, mouse, and gamepad queries.
///
/// - WASD / arrow keys move a square
/// - Left mouse button held: square turns red
/// - Space: prints a one-shot message
/// - Gamepad left stick: moves the square
use nene::{
    input::{GamepadAxis, Key, MouseButton},
    math::{Mat4, Vec2, Vec3},
    renderer::{Context, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer},
    uniform, vertex,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct Transform {
    mvp:   mat4x4<f32>,
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Transform;

@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return u.mvp * vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
"#;

#[vertex]
struct Vert {
    pos: [f32; 2],
}

#[uniform]
struct Transform {
    mvp: [[f32; 4]; 4],
    color: [f32; 4],
}

struct State {
    pipeline: Pipeline,
    vb: VertexBuffer,
    uniform: UniformBuffer,
    pos: Vec2,
}

const QUAD: &[Vert] = &[
    Vert { pos: [-0.5, -0.5] },
    Vert { pos: [0.5, -0.5] },
    Vert { pos: [0.5, 0.5] },
    Vert { pos: [-0.5, -0.5] },
    Vert { pos: [0.5, 0.5] },
    Vert { pos: [-0.5, 0.5] },
];

fn ortho() -> Mat4 {
    Mat4::orthographic_rh(-8.0, 8.0, -4.5, 4.5, -1.0, 1.0)
}

fn build_transform(pos: Vec2, color: [f32; 4]) -> Transform {
    let mvp = ortho() * Mat4::from_translation(Vec3::new(pos.x, pos.y, 0.0));
    Transform {
        mvp: mvp.to_cols_array_2d(),
        color,
    }
}

fn init(ctx: &mut Context) -> State {
    let vb = ctx.create_vertex_buffer(QUAD);
    let uniform = ctx.create_uniform_buffer(&build_transform(Vec2::ZERO, [0.3, 0.6, 1.0, 1.0]));
    let pipeline =
        ctx.create_pipeline(PipelineDescriptor::new(SHADER, Vert::layout()).with_uniform());
    State {
        pipeline,
        vb,
        uniform,
        pos: Vec2::ZERO,
    }
}

fn main() {
    Window::new(Config {
        title: "Input Demo".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            let speed = 5.0 * time.delta;

            // Keyboard movement
            let mut dir = Vec2::ZERO;
            if input.key_down(Key::KeyW) || input.key_down(Key::ArrowUp) {
                dir.y += 1.0;
            }
            if input.key_down(Key::KeyS) || input.key_down(Key::ArrowDown) {
                dir.y -= 1.0;
            }
            if input.key_down(Key::KeyA) || input.key_down(Key::ArrowLeft) {
                dir.x -= 1.0;
            }
            if input.key_down(Key::KeyD) || input.key_down(Key::ArrowRight) {
                dir.x += 1.0;
            }

            // Gamepad left stick
            if let Some((id, _)) = input.gamepads().next() {
                dir.x += input.gamepad_axis(id, GamepadAxis::LeftStickX);
                dir.y += input.gamepad_axis(id, GamepadAxis::LeftStickY);
            }

            if dir != Vec2::ZERO {
                state.pos += dir.normalize() * speed;
            }

            // One-shot key press
            if input.key_pressed(Key::Space) {
                println!("Space pressed! pos = {:?}", state.pos);
            }

            // Color: red while left mouse held, default otherwise
            let color = if input.mouse_down(MouseButton::Left) {
                [1.0, 0.2, 0.2, 1.0]
            } else {
                [0.3, 0.6, 1.0, 1.0]
            };

            ctx.update_uniform_buffer(&state.uniform, &build_transform(state.pos, color));
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.uniform);
            pass.set_vertex_buffer(0, &state.vb);
            pass.draw(0..6);
        },
    );
}
