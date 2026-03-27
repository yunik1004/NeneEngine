//! Input and event bus demo.
//!
//! Shows two complementary input patterns side by side:
//!
//!  1. **Direct queries** — poll `Input` each frame (square movement + mouse tint)
//!  2. **Event bus**      — decouple systems with `Events<E>` (HP / combat log)
//!
//! Controls
//! --------
//! WASD / arrows  — move the square   (direct input query)
//! LMB held       — tint square red   (direct mouse query)
//! Space          — print position to stdout
//! Z              — attack  –10 HP    (emits event → combat system)
//! X              — heal   +15 HP     (emits event → combat system)
//! R              — reset HP          (emits event → combat system)

use nene::{
    event::Events,
    input::{GamepadAxis, Key, MouseButton},
    math::{Mat4, Vec2, Vec3, Vec4},
    renderer::{Context, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer},
    ui::Ui,
    uniform, vertex,
    window::{Config, Window},
};

// ── Shader ────────────────────────────────────────────────────────────────────

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
    mvp: Mat4,
    color: Vec4,
}

// ── Event types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum GameEvent {
    Attack(i32),
    Heal(i32),
    Died,
    Reset,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    // Direct-input rendering
    pipeline: Pipeline,
    vb: VertexBuffer,
    uniform: UniformBuffer,
    pos: Vec2,

    // Event bus
    events: Events<GameEvent>,
    hp: i32,
    log: Vec<String>,

    ui: Ui,
}

const MAX_HP: i32 = 100;
const LOG_MAX: usize = 8;
const W: u32 = 860;
const H: u32 = 480;

const QUAD: &[Vert] = &[
    Vert { pos: [-0.5, -0.5] },
    Vert { pos: [0.5, -0.5] },
    Vert { pos: [0.5, 0.5] },
    Vert { pos: [-0.5, -0.5] },
    Vert { pos: [0.5, 0.5] },
    Vert { pos: [-0.5, 0.5] },
];

// Orthographic projection that maps roughly -8..8 x -4.5..4.5 in world space.
fn ortho() -> Mat4 {
    Mat4::orthographic_rh(-8.0, 8.0, -4.5, 4.5, -1.0, 1.0)
}

fn build_transform(pos: Vec2, color: Vec4) -> Transform {
    let mvp = ortho() * Mat4::from_translation(Vec3::new(pos.x, pos.y, 0.0));
    Transform { mvp, color }
}

fn init(ctx: &mut Context) -> State {
    let vb = ctx.create_vertex_buffer(QUAD);
    let uniform =
        ctx.create_uniform_buffer(&build_transform(Vec2::ZERO, Vec4::new(0.3, 0.6, 1.0, 1.0)));
    let pipeline =
        ctx.create_pipeline(PipelineDescriptor::new(SHADER, Vert::layout()).with_uniform());

    State {
        pipeline,
        vb,
        uniform,
        pos: Vec2::ZERO,
        events: Events::new(),
        hp: MAX_HP,
        log: vec!["Ready.".into()],
        ui: Ui::new(ctx),
    }
}

fn main() {
    Window::new(Config {
        title: "Input + Event bus  (WASD=move  Z=attack  X=heal  R=reset)".to_string(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            // ── advance event buffers ────────────────────────────────────────
            state.events.update();

            // ── combat system (reads previous frame's events) ────────────────
            let mut secondary: Vec<GameEvent> = Vec::new();
            for ev in state.events.read() {
                match ev {
                    GameEvent::Attack(dmg) => {
                        state.hp = (state.hp - dmg).max(0);
                        push_log(&mut state.log, format!("Attack -{dmg}  HP={}", state.hp));
                        if state.hp == 0 {
                            secondary.push(GameEvent::Died);
                        }
                    }
                    GameEvent::Heal(amt) => {
                        state.hp = (state.hp + amt).min(MAX_HP);
                        push_log(&mut state.log, format!("Heal  +{amt}  HP={}", state.hp));
                    }
                    GameEvent::Reset => {
                        state.hp = MAX_HP;
                        push_log(&mut state.log, "Reset → HP=100".into());
                    }
                    GameEvent::Died => {
                        push_log(&mut state.log, "*** DIED ***".into());
                    }
                }
            }
            for ev in secondary {
                state.events.emit(ev);
            }

            // ── input system: direct queries ─────────────────────────────────
            let speed = 5.0 * time.delta;
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
            if let Some((id, _)) = input.gamepads().next() {
                dir.x += input.gamepad_axis(id, GamepadAxis::LeftStickX);
                dir.y += input.gamepad_axis(id, GamepadAxis::LeftStickY);
            }
            if dir != Vec2::ZERO {
                state.pos += dir.normalize() * speed;
            }
            if input.key_pressed(Key::Space) {
                println!("pos = {:?}", state.pos);
            }

            let color = if input.mouse_down(MouseButton::Left) {
                Vec4::new(1.0, 0.2, 0.2, 1.0)
            } else {
                Vec4::new(0.3, 0.6, 1.0, 1.0)
            };
            ctx.update_uniform_buffer(&state.uniform, &build_transform(state.pos, color));

            // ── input system: event emitters ─────────────────────────────────
            if input.key_pressed(Key::KeyZ) {
                state.events.emit(GameEvent::Attack(10));
            }
            if input.key_pressed(Key::KeyX) {
                state.events.emit(GameEvent::Heal(15));
            }
            if input.key_pressed(Key::KeyR) {
                state.events.emit(GameEvent::Reset);
            }

            // ── UI ───────────────────────────────────────────────────────────
            state.ui.begin_frame(input, W as f32, H as f32);

            state.ui.begin_panel("Status", 20.0, 20.0, 200.0);
            state.ui.label("Event Bus");
            state.ui.separator();
            let bar = hp_bar(state.hp, MAX_HP);
            state
                .ui
                .label_dim(&format!("HP  {bar} {}/{MAX_HP}", state.hp));
            state.ui.separator();
            state.ui.label_dim("Z  attack  -10");
            state.ui.label_dim("X  heal   +15");
            state.ui.label_dim("R  reset");
            state.ui.end_panel();

            state.ui.begin_panel("Log", 240.0, 20.0, 220.0);
            state.ui.label("Event log");
            state.ui.separator();
            for line in &state.log {
                state.ui.label_dim(line);
            }
            state.ui.end_panel();

            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_uniform(0, &state.uniform);
            pass.set_vertex_buffer(0, &state.vb);
            pass.draw(0..6);
            state.ui.render(pass);
        },
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn push_log(log: &mut Vec<String>, msg: String) {
    log.push(msg);
    if log.len() > LOG_MAX {
        log.remove(0);
    }
}

fn hp_bar(hp: i32, max: i32) -> String {
    let filled = (hp * 10 / max) as usize;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
