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
    app::{App, WindowId, run},
    event::Events,
    input::{GamepadAxis, Input, Key, MouseButton},
    math::{Mat4, Vec2, Vec3, Vec4},
    renderer::{Context, FlatObject, Pos2, RenderPass},
    time::Time,
    ui::Ui,
    window::Config,
};

// ── Event types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum GameEvent {
    Attack(i32),
    Heal(i32),
    Died,
    Reset,
}

// ── App state ─────────────────────────────────────────────────────────────────

const MAX_HP: i32 = 100;
const LOG_MAX: usize = 8;
const W: u32 = 860;
const H: u32 = 480;

const QUAD: &[Pos2] = &[
    Pos2 { pos: [-0.5, -0.5] },
    Pos2 { pos: [0.5, -0.5] },
    Pos2 { pos: [0.5, 0.5] },
    Pos2 { pos: [-0.5, -0.5] },
    Pos2 { pos: [0.5, 0.5] },
    Pos2 { pos: [-0.5, 0.5] },
];

fn ortho() -> Mat4 {
    Mat4::orthographic_rh(-8.0, 8.0, -4.5, 4.5, -1.0, 1.0)
}

struct InputDemo {
    // Direct-input state
    pos: Vec2,
    color: Vec4,
    // Event bus
    events: Events<GameEvent>,
    hp: i32,
    log: Vec<String>,
    // GPU
    square: Option<FlatObject>,
    ui: Option<Ui>,
}

impl App for InputDemo {
    fn new() -> Self {
        InputDemo {
            pos: Vec2::ZERO,
            color: Vec4::new(0.3, 0.6, 1.0, 1.0),
            events: Events::new(),
            hp: MAX_HP,
            log: vec!["Ready.".into()],
            square: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.square = Some(FlatObject::new(ctx, QUAD, Vec4::new(0.3, 0.6, 1.0, 1.0)));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        // ── advance event buffers ────────────────────────────────────────
        self.events.update();

        // ── combat system ────────────────────────────────────────────────
        let mut secondary: Vec<GameEvent> = Vec::new();
        for ev in self.events.read() {
            match ev {
                GameEvent::Attack(dmg) => {
                    self.hp = (self.hp - dmg).max(0);
                    push_log(&mut self.log, format!("Attack -{dmg}  HP={}", self.hp));
                    if self.hp == 0 {
                        secondary.push(GameEvent::Died);
                    }
                }
                GameEvent::Heal(amt) => {
                    self.hp = (self.hp + amt).min(MAX_HP);
                    push_log(&mut self.log, format!("Heal  +{amt}  HP={}", self.hp));
                }
                GameEvent::Reset => {
                    self.hp = MAX_HP;
                    push_log(&mut self.log, "Reset → HP=100".into());
                }
                GameEvent::Died => {
                    push_log(&mut self.log, "*** DIED ***".into());
                }
            }
        }
        for ev in secondary {
            self.events.emit(ev);
        }

        // ── movement ─────────────────────────────────────────────────────
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
            self.pos += dir.normalize() * speed;
        }
        if input.key_pressed(Key::Space) {
            println!("pos = {:?}", self.pos);
        }

        self.color = if input.mouse_down(MouseButton::Left) {
            Vec4::new(1.0, 0.2, 0.2, 1.0)
        } else {
            Vec4::new(0.3, 0.6, 1.0, 1.0)
        };

        // ── event emitters ────────────────────────────────────────────────
        if input.key_pressed(Key::KeyZ) {
            self.events.emit(GameEvent::Attack(10));
        }
        if input.key_pressed(Key::KeyX) {
            self.events.emit(GameEvent::Heal(15));
        }
        if input.key_pressed(Key::KeyR) {
            self.events.emit(GameEvent::Reset);
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, input: &Input) {
        if let Some(square) = &mut self.square {
            square.color = self.color;
            let mvp = ortho() * Mat4::from_translation(Vec3::new(self.pos.x, self.pos.y, 0.0));
            square.set_transform(ctx, mvp);
        }

        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, W as f32, H as f32);

        ui.begin_panel("Status", 20.0, 20.0, 200.0);
        ui.label("Event Bus");
        ui.separator();
        let bar = hp_bar(self.hp, MAX_HP);
        ui.label_dim(&format!("HP  {bar} {}/{MAX_HP}", self.hp));
        ui.separator();
        ui.label_dim("Z  attack  -10");
        ui.label_dim("X  heal   +15");
        ui.label_dim("R  reset");
        ui.end_panel();

        ui.begin_panel("Log", 240.0, 20.0, 220.0);
        ui.label("Event log");
        ui.separator();
        for line in &self.log {
            ui.label_dim(line);
        }
        ui.end_panel();

        ui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(square) = &self.square {
            square.render(pass);
        }
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Input + Event bus  (WASD=move  Z=attack  X=heal  R=reset)",
            width: W,
            height: H,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<InputDemo>();
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
