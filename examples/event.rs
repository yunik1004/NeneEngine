//! Event bus demo.
//!
//! Shows how `Events<E>` decouples game systems. Three systems share one
//! event queue:
//!
//!  - **Input system** — emits `GameEvent::Attack` / `GameEvent::Heal` on key press
//!  - **Combat system** — reads events, updates HP, emits `GameEvent::Died` if HP ≤ 0
//!  - **UI system**    — reads events and shows a log of what happened
//!
//! Controls
//! --------
//! A   — attack (–10 HP)
//! H   — heal   (+15 HP)
//! R   — reset HP

use nene::{
    event::Events,
    input::Key,
    renderer::{Context, RenderPass},
    ui::Ui,
    window::{Config, Window},
};

const W: u32 = 480;
const H: u32 = 400;
const MAX_HP: i32 = 100;
const LOG_MAX: usize = 8;

#[derive(Debug, Clone)]
enum GameEvent {
    Attack(i32),
    Heal(i32),
    Died,
    Reset,
}

struct State {
    events: Events<GameEvent>,
    hp: i32,
    log: Vec<String>,
    ui: Ui,
}

fn init(ctx: &mut Context) -> State {
    State {
        events: Events::new(),
        hp: MAX_HP,
        log: vec!["Ready.".into()],
        ui: Ui::new(ctx),
    }
}

fn main() {
    Window::new(Config {
        title: "Event demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, _ctx, input, _time| {
            // ── advance buffers ──────────────────────────────────────────────
            // update() first: last frame's current → old. Combat reads old.
            // Then input emits into fresh current, visible next frame.
            state.events.update();

            // ── combat system (reads previous frame's events) ─────────────
            let mut new_events: Vec<GameEvent> = Vec::new();
            for ev in state.events.read() {
                match ev {
                    GameEvent::Attack(dmg) => {
                        state.hp = (state.hp - dmg).max(0);
                        push_log(&mut state.log, format!("Attack -{dmg}  HP={}", state.hp));
                        if state.hp == 0 {
                            new_events.push(GameEvent::Died);
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
            for ev in new_events {
                state.events.emit(ev);
            }

            // ── input system (emits into current, processed next frame) ───
            if input.key_pressed(Key::KeyA) {
                state.events.emit(GameEvent::Attack(10));
            }
            if input.key_pressed(Key::KeyH) {
                state.events.emit(GameEvent::Heal(15));
            }
            if input.key_pressed(Key::KeyR) {
                state.events.emit(GameEvent::Reset);
            }

            // ── UI ───────────────────────────────────────────────────────────
            state.ui.begin_frame(input, W as f32, H as f32);

            state.ui.begin_panel("Status", 20.0, 20.0, 200.0);
            state.ui.label("Status");
            state.ui.separator();
            let bar = hp_bar(state.hp, MAX_HP);
            state
                .ui
                .label_dim(&format!("HP  {bar} {}/{MAX_HP}", state.hp));
            state.ui.separator();
            state.ui.label_dim("A  attack  -10");
            state.ui.label_dim("H  heal   +15");
            state.ui.label_dim("R  reset");
            state.ui.end_panel();

            state.ui.begin_panel("Log", 240.0, 20.0, 220.0);
            state.ui.label("Event log");
            state.ui.separator();
            for line in &state.log {
                state.ui.label_dim(line);
            }
            state.ui.end_panel();

            state.ui.end_frame(_ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.ui.render(pass);
        },
    );
}

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
