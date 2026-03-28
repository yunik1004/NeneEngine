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
    app::{App, WindowId, run},
    input::{Input, Key},
    locale::{Locale, from_json},
    persist::{SaveStore, Settings},
    renderer::{Context, RenderPass},
    time::Time,
    ui::Ui,
    window::Config,
};

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

// ── App ───────────────────────────────────────────────────────────────────────

const W: f32 = 980.0;
const H: f32 = 580.0;
const SLOT: &str = "save1";

struct UiDemo {
    ui: Option<Ui>,

    speed: f32,
    intensity: f32,
    brightness: f32,
    show_fps: bool,
    settings: Settings,
    settings_msg: String,

    store: SaveStore,
    game: GameState,
    save_msg: String,

    locale: Locale,
    langs: Vec<String>,
    lang_idx: usize,
}

impl App for UiDemo {
    fn new() -> Self {
        let tmp = std::env::temp_dir().join("nene_ui_example");
        let mut settings = Settings::new(tmp.join("settings.json"));
        settings.register("sim.speed", 3.5f32);
        settings.register("sim.intensity", 0.7f32);
        settings.register("sim.brightness", 1.0f32);
        settings.register("display.show_fps", true);

        let settings_msg = if settings.exists() {
            "loaded from disk"
        } else {
            "using defaults"
        };

        let speed: f32 = settings.get("sim.speed").unwrap_or(3.5);
        let intensity: f32 = settings.get("sim.intensity").unwrap_or(0.7);
        let brightness: f32 = settings.get("sim.brightness").unwrap_or(1.0);
        let show_fps: bool = settings.get("display.show_fps").unwrap_or(true);

        let locale_dir = "examples/assets/locale";
        let langs = available_languages(locale_dir);
        let locale = Locale::new(load_lang(locale_dir, "en"));

        UiDemo {
            ui: None,
            speed,
            intensity,
            brightness,
            show_fps,
            settings,
            settings_msg: settings_msg.into(),
            store: SaveStore::new(&tmp),
            game: GameState::default(),
            save_msg: "no save loaded".into(),
            locale,
            langs,
            lang_idx: 0,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        if input.key_pressed(Key::KeyL) && !self.langs.is_empty() {
            self.lang_idx = (self.lang_idx + 1) % self.langs.len();
            let lang = self.langs[self.lang_idx].clone();
            self.locale.set(load_lang("examples/assets/locale", &lang));
        }

        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, W, H);

        // ── Left panel: Settings ──────────────────────────────────────────────
        ui.begin_panel("Settings", 20.0, 20.0, 240.0);
        ui.label("Parameters");
        ui.separator();
        ui.slider("Speed", &mut self.speed, 0.0, 10.0);
        ui.slider("Intensity", &mut self.intensity, 0.0, 1.0);
        ui.slider("Brightness", &mut self.brightness, 0.0, 2.0);
        ui.separator();
        ui.checkbox("Show FPS", &mut self.show_fps);
        ui.separator();
        if ui.button("Save Settings") {
            let _ = self.settings.set("sim.speed", &self.speed);
            let _ = self.settings.set("sim.intensity", &self.intensity);
            let _ = self.settings.set("sim.brightness", &self.brightness);
            let _ = self.settings.set("display.show_fps", &self.show_fps);
            self.settings_msg = match self.settings.save() {
                Ok(_) => "saved to disk".into(),
                Err(e) => format!("error: {e}"),
            };
        }
        ui.label_dim(&self.settings_msg.clone());
        ui.end_panel();

        // ── Centre panel: Readout ─────────────────────────────────────────────
        ui.begin_panel("Readout", 280.0, 20.0, 200.0);
        ui.label("Live values");
        ui.separator();
        ui.label_dim(&format!("speed      {:.2}", self.speed));
        ui.label_dim(&format!("intensity  {:.2}", self.intensity));
        ui.label_dim(&format!("brightness {:.2}", self.brightness));
        ui.separator();
        if self.show_fps {
            let fps = if time.delta > 0.0 {
                (1.0 / time.delta) as u32
            } else {
                0
            };
            ui.label(&format!("FPS  {fps}"));
        } else {
            ui.label_dim("FPS  —");
        }
        ui.end_panel();

        // ── Centre-right panel: Save slots ────────────────────────────────────
        ui.begin_panel("Save Game", 500.0, 20.0, 200.0);
        ui.label_dim(&format!("level  {}", self.game.level));
        ui.label_dim(&format!("score  {}", self.game.score));
        ui.separator();
        if ui.button("+10 Score") {
            self.game.score += 10;
        }
        if ui.button("Level Up") {
            self.game.level += 1;
        }
        ui.separator();
        if ui.button("Save Game") {
            self.save_msg = match self
                .store
                .set(SLOT, "game", &self.game)
                .and_then(|_| self.store.flush(SLOT))
            {
                Ok(_) => format!("saved → {SLOT}.json"),
                Err(e) => format!("error: {e}"),
            };
        }
        if ui.button("Load Game") {
            match self.store.get::<GameState>(SLOT, "game") {
                Some(g) => {
                    self.game = g;
                    self.save_msg = format!("loaded {SLOT}.json");
                }
                None => self.save_msg = "no save found".into(),
            }
        }
        if ui.button("Delete Save") {
            self.save_msg = match self.store.delete(SLOT) {
                Ok(_) => format!("deleted {SLOT}.json"),
                Err(e) => format!("error: {e}"),
            };
        }
        ui.separator();
        ui.label_dim(&self.save_msg.clone());
        ui.end_panel();

        // ── Right panel: Locale ───────────────────────────────────────────────
        let score_str = self.game.score.to_string();
        ui.begin_panel("Locale", 720.0, 20.0, 220.0);
        let lang = self
            .langs
            .get(self.lang_idx)
            .map(|s| s.as_str())
            .unwrap_or("en");
        ui.label(&format!("Language: {lang}"));
        ui.separator();
        ui.label(&self.locale.t("menu.start"));
        ui.label(&self.locale.t("menu.quit"));
        ui.separator();
        ui.label(&self.locale.t_with("hud.score", &[("score", &score_str)]));
        ui.label(&self.locale.t_with("dialog.greet", &[("name", "Player")]));
        ui.separator();
        ui.label_dim("L  cycle language");
        ui.end_panel();
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        if let Some(ui) = &mut self.ui {
            ui.end_frame(ctx);
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "UI · Persistence · Locale  (L = cycle language)",
            width: W as u32,
            height: H as u32,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<UiDemo>();
}

fn load_lang(dir: &str, lang: &str) -> std::collections::HashMap<String, String> {
    let path = format!("{dir}/{lang}.json");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    from_json(&text)
}

fn available_languages(dir: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut langs: Vec<String> = rd
        .filter_map(|e| {
            let p = e.ok()?.path();
            if p.extension()?.to_str()? == "json" {
                Some(p.file_stem()?.to_str()?.to_owned())
            } else {
                None
            }
        })
        .collect();
    langs.sort();
    langs
}
