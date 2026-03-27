//! Localization demo.
//!
//! Shows a simple UI panel with translated strings.
//! Press L to cycle through available languages.
//!
//! Usage: cargo run --example locale

use nene::{
    input::Key,
    locale::Locale,
    renderer::{Context, RenderPass},
    ui::Ui,
    window::{Config, Window},
};

const W: u32 = 480;
const H: u32 = 320;

struct State {
    locale: Locale,
    ui: Ui,
    langs: Vec<String>,
    lang_idx: usize,
    hp: u32,
    score: u32,
}

fn init(ctx: &mut Context) -> State {
    let locale = Locale::new("examples/assets/locale", "en");
    let langs = locale.available_languages();
    State {
        locale,
        ui: Ui::new(ctx),
        langs,
        lang_idx: 0,
        hp: 75,
        score: 1234,
    }
}

fn main() {
    Window::new(Config {
        title: "Locale demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, _ctx, input, _time| {
            // Cycle language on L press
            if input.key_pressed(Key::KeyL) && !state.langs.is_empty() {
                state.lang_idx = (state.lang_idx + 1) % state.langs.len();
                let lang = state.langs[state.lang_idx].clone();
                state.locale.set_language(&lang);
            }

            let l = &state.locale;
            let hp_str = state.hp.to_string();
            let score_str = state.score.to_string();

            state.ui.begin_frame(input, W as f32, H as f32);
            state.ui.begin_panel("Locale", 20.0, 20.0, 280.0);

            state.ui.label(&format!("Language: {}", l.language()));
            state.ui.label(&l.t("menu.start"));
            state.ui.label(&l.t("menu.quit"));
            state.ui.label(&l.t_with("hud.hp", &[("hp", &hp_str)]));
            state
                .ui
                .label(&l.t_with("hud.score", &[("score", &score_str)]));
            state
                .ui
                .label(&l.t_with("dialog.greet", &[("name", "Player")]));
            state.ui.label_dim("(Press L to switch language)");

            state.ui.end_panel();
            state.ui.end_frame(_ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.ui.render(pass);
        },
    );
}
