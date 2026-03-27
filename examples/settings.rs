//! Settings demo.
//!
//! Demonstrates loading, modifying, and saving game settings.
//! Changes persist to `saves/settings.json` between runs.
//!
//! Controls
//! --------
//! ↑ / ↓      — adjust master volume
//! ← / →      — adjust music volume
//! F           — toggle fullscreen setting
//! S           — save to disk
//! R           — reset all to defaults

use nene::{
    input::{Input, Key},
    renderer::{Context, RenderPass},
    settings::Settings,
    ui::Ui,
    window::{Config, Window},
};

// applied each frame to sync window state with settings
fn apply_window_settings(ctx: &Context, settings: &mut Settings) {
    let fullscreen: bool = settings.get("video.fullscreen").unwrap_or(false);
    ctx.set_fullscreen(fullscreen);
}

const W: u32 = 520;
const H: u32 = 420;

struct State {
    settings: Settings,
    ui: Ui,
    message: String,
}

fn init(ctx: &mut Context) -> State {
    let mut settings = Settings::new("saves/settings.json");

    // Register defaults (applied only when the key is absent).
    settings.register("audio.master_volume", 1.0f32);
    settings.register("audio.music_volume", 0.8f32);
    settings.register("audio.sfx_volume", 1.0f32);
    settings.register("video.fullscreen", false);
    settings.register("video.vsync", true);

    let msg = if settings.exists() {
        "loaded from disk".into()
    } else {
        "using defaults".into()
    };

    State {
        settings,
        ui: Ui::new(ctx),
        message: msg,
    }
}

fn main() {
    Window::new(Config {
        title: "Settings demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, _time| {
            handle_input(state, input);
            apply_window_settings(ctx, &mut state.settings);

            let master: f32 = state.settings.get("audio.master_volume").unwrap_or(1.0);
            let music: f32 = state.settings.get("audio.music_volume").unwrap_or(0.8);
            let sfx: f32 = state.settings.get("audio.sfx_volume").unwrap_or(1.0);
            let fullscreen: bool = state.settings.get("video.fullscreen").unwrap_or(false);
            let vsync: bool = state.settings.get("video.vsync").unwrap_or(true);

            state.ui.begin_frame(input, W as f32, H as f32);

            state.ui.begin_panel("Audio", 20.0, 20.0, 220.0);
            state.ui.label("Audio");
            state.ui.separator();
            state.ui.label_dim(&format!("master  {:.2}", master));
            state.ui.label_dim(&format!("music   {:.2}", music));
            state.ui.label_dim(&format!("sfx     {:.2}", sfx));
            state.ui.end_panel();

            state.ui.begin_panel("Video", 20.0, 200.0, 220.0);
            state.ui.label("Video");
            state.ui.separator();
            state.ui.label_dim(&format!("fullscreen  {}", fullscreen));
            state.ui.label_dim(&format!("vsync       {}", vsync));
            state.ui.end_panel();

            state.ui.begin_panel("Controls", 260.0, 20.0, 220.0);
            state.ui.label("Keys");
            state.ui.separator();
            state.ui.label_dim("↑ / ↓   master vol");
            state.ui.label_dim("← / →   music vol");
            state.ui.label_dim("F       fullscreen");
            state.ui.separator();
            state.ui.label_dim("S       save");
            state.ui.label_dim("R       reset all");
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
    let step = 0.05f32;

    if input.key_pressed(Key::ArrowUp) {
        let v: f32 = state.settings.get("audio.master_volume").unwrap_or(1.0);
        state
            .settings
            .set("audio.master_volume", &(v + step).min(1.0))
            .unwrap();
    }
    if input.key_pressed(Key::ArrowDown) {
        let v: f32 = state.settings.get("audio.master_volume").unwrap_or(1.0);
        state
            .settings
            .set("audio.master_volume", &(v - step).max(0.0))
            .unwrap();
    }
    if input.key_pressed(Key::ArrowRight) {
        let v: f32 = state.settings.get("audio.music_volume").unwrap_or(0.8);
        state
            .settings
            .set("audio.music_volume", &(v + step).min(1.0))
            .unwrap();
    }
    if input.key_pressed(Key::ArrowLeft) {
        let v: f32 = state.settings.get("audio.music_volume").unwrap_or(0.8);
        state
            .settings
            .set("audio.music_volume", &(v - step).max(0.0))
            .unwrap();
    }
    if input.key_pressed(Key::KeyF) {
        let v: bool = state.settings.get("video.fullscreen").unwrap_or(false);
        state.settings.set("video.fullscreen", &!v).unwrap();
    }
    if input.key_pressed(Key::KeyS) {
        match state.settings.save() {
            Ok(_) => state.message = "saved to saves/settings.json".into(),
            Err(e) => state.message = format!("error: {e}"),
        }
    }
    if input.key_pressed(Key::KeyR) {
        state.settings.reset_all();
        state.message = "reset to defaults".into();
    }
}
