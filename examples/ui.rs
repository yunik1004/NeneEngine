//! UI, persistence, and localization demo.
//!
//! Shows how immediate-mode widgets connect directly to persistent storage:
//!
//!  • Sliders / checkbox → [`Settings`] (typed key-value, saved to disk)
//!  • Buttons            → [`SaveStore`] (slot-based save / load / delete)
//!  • Locale panel       → [`Locale`] (L to cycle language)
//!
//! Controls
//! --------
//! Interact with the widgets using the mouse.
//! L — cycle language in the Locale panel.

use nene::{
    input::Key,
    locale::Locale,
    persist::{SaveStore, Settings},
    renderer::{Context, RenderPass},
    ui::Ui,
    window::{Config, Window},
};

// ── Background quad ───────────────────────────────────────────────────────────

const BG_SHADER: &str = r#"
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0), vec2<f32>(-1.0, 1.0), vec2<f32>(3.0, 1.0),
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}
@fragment fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.08, 0.08, 0.10, 1.0);
}
"#;

// ── Saved game state ──────────────────────────────────────────────────────────

#[nene::data]
struct GameState {
    level: u32,
    score: u32,
}

impl Default for GameState {
    fn default() -> Self {
        Self { level: 1, score: 0 }
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    ui: Ui,
    bg_pipeline: nene::renderer::Pipeline,

    // Settings-backed widget values
    speed: f32,
    intensity: f32,
    brightness: f32,
    show_fps: bool,

    settings: Settings,
    settings_msg: String,

    // SaveStore-backed game state
    store: SaveStore,
    game: GameState,
    save_msg: String,

    // Locale
    locale: Locale,
    langs: Vec<String>,
    lang_idx: usize,
}

const W: f32 = 980.0;
const H: f32 = 580.0;
const SLOT: &str = "save1";

fn init(ctx: &mut Context) -> State {
    let bg_pipeline = ctx.create_pipeline(nene::renderer::PipelineDescriptor::fullscreen_pass(
        BG_SHADER,
    ));

    let tmp = std::env::temp_dir().join("nene_ui_example");
    let mut settings = Settings::new(tmp.join("settings.json"));
    settings.register("sim.speed", 3.5f32);
    settings.register("sim.intensity", 0.7f32);
    settings.register("sim.brightness", 1.0f32);
    settings.register("display.show_fps", true);

    let settings_msg = if settings.exists() {
        "loaded from disk".into()
    } else {
        "using defaults".into()
    };

    let speed: f32 = settings.get("sim.speed").unwrap_or(3.5);
    let intensity: f32 = settings.get("sim.intensity").unwrap_or(0.7);
    let brightness: f32 = settings.get("sim.brightness").unwrap_or(1.0);
    let show_fps: bool = settings.get("display.show_fps").unwrap_or(true);

    let locale = Locale::new("examples/assets/locale", "en");
    let langs = locale.available_languages();

    State {
        ui: Ui::new(ctx),
        bg_pipeline,
        speed,
        intensity,
        brightness,
        show_fps,
        settings,
        settings_msg,
        store: SaveStore::new(&tmp),
        game: GameState::default(),
        save_msg: "no save loaded".into(),
        locale,
        langs,
        lang_idx: 0,
    }
}

fn main() {
    Window::new(Config {
        title: "UI · Persistence · Locale  (L = cycle language)".to_string(),
        width: W as u32,
        height: H as u32,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            if input.key_pressed(Key::KeyL) && !state.langs.is_empty() {
                state.lang_idx = (state.lang_idx + 1) % state.langs.len();
                let lang = state.langs[state.lang_idx].clone();
                state.locale.set_language(&lang);
            }

            state.ui.begin_frame(input, W, H);

            // ── Left panel: Settings ──────────────────────────────────────────
            state.ui.begin_panel("Settings", 20.0, 20.0, 240.0);
            state.ui.label("Parameters");
            state.ui.separator();
            state.ui.slider("Speed", &mut state.speed, 0.0, 10.0);
            state.ui.slider("Intensity", &mut state.intensity, 0.0, 1.0);
            state
                .ui
                .slider("Brightness", &mut state.brightness, 0.0, 2.0);
            state.ui.separator();
            state.ui.checkbox("Show FPS", &mut state.show_fps);
            state.ui.separator();
            if state.ui.button("Save Settings") {
                let _ = state.settings.set("sim.speed", &state.speed);
                let _ = state.settings.set("sim.intensity", &state.intensity);
                let _ = state.settings.set("sim.brightness", &state.brightness);
                let _ = state.settings.set("display.show_fps", &state.show_fps);
                state.settings_msg = match state.settings.save() {
                    Ok(_) => "saved to disk".into(),
                    Err(e) => format!("error: {e}"),
                };
            }
            state.ui.label_dim(&state.settings_msg.clone());
            state.ui.end_panel();

            // ── Centre panel: Readout ─────────────────────────────────────────
            state.ui.begin_panel("Readout", 280.0, 20.0, 200.0);
            state.ui.label("Live values");
            state.ui.separator();
            state
                .ui
                .label_dim(&format!("speed      {:.2}", state.speed));
            state
                .ui
                .label_dim(&format!("intensity  {:.2}", state.intensity));
            state
                .ui
                .label_dim(&format!("brightness {:.2}", state.brightness));
            state.ui.separator();
            if state.show_fps {
                let fps = if time.delta > 0.0 {
                    (1.0 / time.delta) as u32
                } else {
                    0
                };
                state.ui.label(&format!("FPS  {fps}"));
            } else {
                state.ui.label_dim("FPS  —");
            }
            state.ui.end_panel();

            // ── Centre-right panel: Save slots ────────────────────────────────
            state.ui.begin_panel("Save Game", 500.0, 20.0, 200.0);
            state.ui.label_dim(&format!("level  {}", state.game.level));
            state.ui.label_dim(&format!("score  {}", state.game.score));
            state.ui.separator();
            if state.ui.button("+10 Score") {
                state.game.score += 10;
            }
            if state.ui.button("Level Up") {
                state.game.level += 1;
            }
            state.ui.separator();
            if state.ui.button("Save Game") {
                state.save_msg = match state
                    .store
                    .set(SLOT, "game", &state.game)
                    .and_then(|_| state.store.flush(SLOT))
                {
                    Ok(_) => format!("saved → {SLOT}.json"),
                    Err(e) => format!("error: {e}"),
                };
            }
            if state.ui.button("Load Game") {
                match state.store.get::<GameState>(SLOT, "game") {
                    Some(g) => {
                        state.game = g;
                        state.save_msg = format!("loaded {SLOT}.json");
                    }
                    None => state.save_msg = "no save found".into(),
                }
            }
            if state.ui.button("Delete Save") {
                state.save_msg = match state.store.delete(SLOT) {
                    Ok(_) => format!("deleted {SLOT}.json"),
                    Err(e) => format!("error: {e}"),
                };
            }
            state.ui.separator();
            state.ui.label_dim(&state.save_msg.clone());
            state.ui.end_panel();

            // ── Right panel: Locale ───────────────────────────────────────────
            let l = &state.locale;
            let score_str = state.game.score.to_string();
            state.ui.begin_panel("Locale", 720.0, 20.0, 220.0);
            state.ui.label(&format!("Language: {}", l.language()));
            state.ui.separator();
            state.ui.label(&l.t("menu.start"));
            state.ui.label(&l.t("menu.quit"));
            state.ui.separator();
            state
                .ui
                .label(&l.t_with("hud.score", &[("score", &score_str)]));
            state
                .ui
                .label(&l.t_with("dialog.greet", &[("name", "Player")]));
            state.ui.separator();
            state.ui.label_dim("L  cycle language");
            state.ui.end_panel();

            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.bg_pipeline);
            pass.draw(0..3);
            state.ui.render(pass);
        },
    );
}
