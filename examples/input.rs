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
    app::{App, Config, WindowEvent, WindowId, run},
    event::Events,
    input::{ActionMap, GamepadAxis, Input, Key, MouseButton},
    math::{Mat4, Vec2, Vec3, Vec4},
    renderer::{Context, FlatObject, RenderPass},
    time::Time,
    ui::Ui,
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

const QUAD: &[Vec2] = &[
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

fn ortho() -> Mat4 {
    Mat4::orthographic_rh(-8.0, 8.0, -4.5, 4.5, -1.0, 1.0)
}

#[derive(Hash, PartialEq, Eq)]
enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    PrintPos,
    Tint,
    Attack,
    Heal,
    Reset,
}

struct InputDemo {
    pos: Vec2,
    color: Vec4,
    events: Events<GameEvent>,
    hp: i32,
    log: Vec<String>,
    bindings: ActionMap<Action>,
    square: Option<FlatObject>,
    egui: Option<Ui>,
}

impl App for InputDemo {
    fn new() -> Self {
        let mut bindings = ActionMap::new();
        bindings
            .bind(Action::MoveUp, Key::KeyW)
            .bind(Action::MoveUp, Key::ArrowUp)
            .bind(Action::MoveDown, Key::KeyS)
            .bind(Action::MoveDown, Key::ArrowDown)
            .bind(Action::MoveLeft, Key::KeyA)
            .bind(Action::MoveLeft, Key::ArrowLeft)
            .bind(Action::MoveRight, Key::KeyD)
            .bind(Action::MoveRight, Key::ArrowRight)
            .bind(Action::PrintPos, Key::Space)
            .bind(Action::Tint, MouseButton::Left)
            .bind(Action::Attack, Key::KeyZ)
            .bind(Action::Heal, Key::KeyX)
            .bind(Action::Reset, Key::KeyR);
        InputDemo {
            pos: Vec2::ZERO,
            color: Vec4::new(0.3, 0.6, 1.0, 1.0),
            events: Events::new(),
            hp: MAX_HP,
            log: vec!["Ready.".into()],
            bindings,
            square: None,
            egui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.square = Some(FlatObject::new(ctx, QUAD, Vec4::new(0.3, 0.6, 1.0, 1.0)));
        self.egui = Some(Ui::new(ctx));
    }

    fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
        if let Some(e) = &mut self.egui {
            e.handle_event(event);
        }
    }

    fn update(&mut self, input: &Input, time: &Time) {
        self.events.update();

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

        let speed = 5.0 * time.delta;
        let mut dir = Vec2::ZERO;
        if self.bindings.down(input, &Action::MoveUp) {
            dir.y += 1.0;
        }
        if self.bindings.down(input, &Action::MoveDown) {
            dir.y -= 1.0;
        }
        if self.bindings.down(input, &Action::MoveLeft) {
            dir.x -= 1.0;
        }
        if self.bindings.down(input, &Action::MoveRight) {
            dir.x += 1.0;
        }
        if let Some((id, _)) = input.gamepads().next() {
            dir.x += input.gamepad_axis(id, GamepadAxis::LeftStickX);
            dir.y += input.gamepad_axis(id, GamepadAxis::LeftStickY);
        }
        if dir != Vec2::ZERO {
            self.pos += dir.normalize() * speed;
        }
        if self.bindings.pressed(input, &Action::PrintPos) {
            println!("pos = {:?}", self.pos);
        }

        self.color = if self.bindings.down(input, &Action::Tint) {
            Vec4::new(1.0, 0.2, 0.2, 1.0)
        } else {
            Vec4::new(0.3, 0.6, 1.0, 1.0)
        };

        if self.bindings.pressed(input, &Action::Attack) {
            self.events.emit(GameEvent::Attack(10));
        }
        if self.bindings.pressed(input, &Action::Heal) {
            self.events.emit(GameEvent::Heal(15));
        }
        if self.bindings.pressed(input, &Action::Reset) {
            self.events.emit(GameEvent::Reset);
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        if let Some(square) = &mut self.square {
            square.color = self.color;
            let mvp = ortho() * Mat4::from_translation(Vec3::new(self.pos.x, self.pos.y, 0.0));
            square.set_transform(ctx, mvp);
        }

        let Some(egui) = &mut self.egui else { return };
        let ui_ctx = egui.begin_frame();

        let bar = hp_bar(self.hp, MAX_HP);
        egui::Window::new("Status")
            .default_pos(egui::pos2(20.0, 20.0))
            .default_width(200.0)
            .resizable(false)
            .show(&ui_ctx, |ui| {
                ui.label("Event Bus");
                ui.separator();
                ui.label(egui::RichText::new(format!("HP  {bar} {}/{MAX_HP}", self.hp)).weak());
                ui.separator();
                ui.label(egui::RichText::new("Z  attack  -10").weak());
                ui.label(egui::RichText::new("X  heal   +15").weak());
                ui.label(egui::RichText::new("R  reset").weak());
            });

        egui::Window::new("Log")
            .default_pos(egui::pos2(240.0, 20.0))
            .default_width(220.0)
            .resizable(false)
            .show(&ui_ctx, |ui| {
                ui.label("Event log");
                ui.separator();
                for line in &self.log {
                    ui.label(egui::RichText::new(line).weak());
                }
            });

        egui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(square) = &self.square {
            square.render(pass);
        }
        if let Some(e) = &self.egui {
            e.render(pass);
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
