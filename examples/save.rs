//! Save / load demo.
//!
//! Simulates a simple game loop where you can:
//!   • Press S — save current state to slot "save1"
//!   • Press L — load from slot "save1"
//!   • Press D — delete slot "save1"
//!   • Press Space — gain 10 score
//!   • Press R — level up
//!
//! Save files are written to "./saves/" relative to the working directory.

use nene::{
    input::{Input, Key},
    renderer::{Context, RenderPass},
    save::SaveStore,
    ui::Ui,
    window::{Config, Window},
};
use serde::{Deserialize, Serialize};

const W: u32 = 600;
const H: u32 = 400;
const SLOT: &str = "save1";

#[derive(Serialize, Deserialize, Clone)]
struct GameState {
    level: u32,
    score: u32,
    player_name: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            level: 1,
            score: 0,
            player_name: "Player".into(),
        }
    }
}

struct State {
    ui: Ui,
    store: SaveStore,
    game: GameState,
    message: String,
}

fn init(ctx: &mut Context) -> State {
    State {
        ui: Ui::new(ctx),
        store: SaveStore::new("saves"),
        game: GameState::default(),
        message: "Press S to save, L to load".into(),
    }
}

fn main() {
    Window::new(Config {
        title: "Save demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, _time| {
            handle_input(state, input);

            state.ui.begin_frame(input, W as f32, H as f32);

            state.ui.begin_panel("Game", 20.0, 20.0, 220.0);
            state.ui.label("State");
            state.ui.separator();
            state
                .ui
                .label_dim(&format!("name   {}", state.game.player_name));
            state.ui.label_dim(&format!("level  {}", state.game.level));
            state.ui.label_dim(&format!("score  {}", state.game.score));
            state.ui.end_panel();

            state.ui.begin_panel("Controls", 260.0, 20.0, 220.0);
            state.ui.label("Keys");
            state.ui.separator();
            state.ui.label_dim("Space  +10 score");
            state.ui.label_dim("R      level up");
            state.ui.separator();
            state.ui.label_dim("S      save");
            state.ui.label_dim("L      load");
            state.ui.label_dim("D      delete save");
            state.ui.separator();
            state.ui.label(&state.message.clone());
            state.ui.end_panel();

            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.ui.render(pass);
        },
    );
}

fn handle_input(state: &mut State, input: &Input) {
    if input.key_pressed(Key::Space) {
        state.game.score += 10;
        state.message = format!("score → {}", state.game.score);
    }

    if input.key_pressed(Key::KeyR) {
        state.game.level += 1;
        state.message = format!("level → {}", state.game.level);
    }

    if input.key_pressed(Key::KeyS) {
        match state.store.set(SLOT, "game", &state.game) {
            Ok(_) => match state.store.flush(SLOT) {
                Ok(_) => state.message = format!("saved to {SLOT}.json"),
                Err(e) => state.message = format!("flush error: {e}"),
            },
            Err(e) => state.message = format!("save error: {e}"),
        }
    }

    if input.key_pressed(Key::KeyL) {
        match state.store.get::<GameState>(SLOT, "game") {
            Some(g) => {
                state.game = g;
                state.message = format!("loaded from {SLOT}.json");
            }
            None => state.message = "no save found".into(),
        }
    }

    if input.key_pressed(Key::KeyD) {
        match state.store.delete(SLOT) {
            Ok(_) => state.message = format!("deleted {SLOT}.json"),
            Err(e) => state.message = format!("delete error: {e}"),
        }
    }
}
