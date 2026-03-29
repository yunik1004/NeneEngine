//! UI, persistence, and localization demo.
//!
//! Shows how egui widgets connect directly to persistent storage:
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
    app::{App, Config, WindowEvent, WindowId, run},
    input::{ActionMap, Input, Key},
    locale::{Locale, from_json},
    persist::{SaveStore, Settings},
    renderer::{Context, RenderPass},
    time::Time,
    ui::EguiUi,
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

#[derive(Hash, PartialEq, Eq)]
enum Action {
    CycleLanguage,
}

struct UiDemo {
    egui: Option<EguiUi>,

    fps: u32,
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
    bindings: ActionMap<Action>,
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

        let mut bindings = ActionMap::new();
        bindings.bind(Action::CycleLanguage, Key::KeyL);

        UiDemo {
            egui: None,
            fps: 0,
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
            bindings,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.egui = Some(EguiUi::new(ctx));
    }

    fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
        if let Some(e) = &mut self.egui {
            e.handle_event(event);
        }
    }

    fn update(&mut self, input: &Input, time: &Time) {
        self.fps = if time.delta > 0.0 {
            (1.0 / time.delta) as u32
        } else {
            0
        };

        if self.bindings.pressed(input, &Action::CycleLanguage) && !self.langs.is_empty() {
            self.lang_idx = (self.lang_idx + 1) % self.langs.len();
            let lang = self.langs[self.lang_idx].clone();
            self.locale.set(load_lang("examples/assets/locale", &lang));
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let Some(egui) = &mut self.egui else { return };
        let ui_ctx = egui.begin_frame();

        // ── Left panel: Settings ──────────────────────────────────────────────
        egui::Window::new("Settings")
            .default_pos(egui::pos2(20.0, 20.0))
            .default_width(240.0)
            .show(&ui_ctx, |ui| {
                ui.label("Parameters");
                ui.separator();
                ui.add(egui::Slider::new(&mut self.speed, 0.0..=10.0).text("Speed"));
                ui.add(egui::Slider::new(&mut self.intensity, 0.0..=1.0).text("Intensity"));
                ui.add(egui::Slider::new(&mut self.brightness, 0.0..=2.0).text("Brightness"));
                ui.separator();
                ui.checkbox(&mut self.show_fps, "Show FPS");
                ui.separator();
                if ui.button("Save Settings").clicked() {
                    let _ = self.settings.set("sim.speed", &self.speed);
                    let _ = self.settings.set("sim.intensity", &self.intensity);
                    let _ = self.settings.set("sim.brightness", &self.brightness);
                    let _ = self.settings.set("display.show_fps", &self.show_fps);
                    self.settings_msg = match self.settings.save() {
                        Ok(_) => "saved to disk".into(),
                        Err(e) => format!("error: {e}"),
                    };
                }
                ui.label(egui::RichText::new(&self.settings_msg).weak());
            });

        // ── Centre panel: Readout ─────────────────────────────────────────────
        egui::Window::new("Readout")
            .default_pos(egui::pos2(280.0, 20.0))
            .default_width(200.0)
            .show(&ui_ctx, |ui| {
                ui.label("Live values");
                ui.separator();
                ui.label(egui::RichText::new(format!("speed      {:.2}", self.speed)).weak());
                ui.label(egui::RichText::new(format!("intensity  {:.2}", self.intensity)).weak());
                ui.label(egui::RichText::new(format!("brightness {:.2}", self.brightness)).weak());
                ui.separator();
                if self.show_fps {
                    ui.label(format!("FPS  {}", self.fps));
                } else {
                    ui.label(egui::RichText::new("FPS  —").weak());
                }
            });

        // ── Centre-right panel: Save slots ────────────────────────────────────
        egui::Window::new("Save Game")
            .default_pos(egui::pos2(500.0, 20.0))
            .default_width(200.0)
            .show(&ui_ctx, |ui| {
                ui.label(egui::RichText::new(format!("level  {}", self.game.level)).weak());
                ui.label(egui::RichText::new(format!("score  {}", self.game.score)).weak());
                ui.separator();
                if ui.button("+10 Score").clicked() {
                    self.game.score += 10;
                }
                if ui.button("Level Up").clicked() {
                    self.game.level += 1;
                }
                ui.separator();
                if ui.button("Save Game").clicked() {
                    self.save_msg = match self
                        .store
                        .set(SLOT, "game", &self.game)
                        .and_then(|_| self.store.flush(SLOT))
                    {
                        Ok(_) => format!("saved → {SLOT}.json"),
                        Err(e) => format!("error: {e}"),
                    };
                }
                if ui.button("Load Game").clicked() {
                    match self.store.get::<GameState>(SLOT, "game") {
                        Some(g) => {
                            self.game = g;
                            self.save_msg = format!("loaded {SLOT}.json");
                        }
                        None => self.save_msg = "no save found".into(),
                    }
                }
                if ui.button("Delete Save").clicked() {
                    self.save_msg = match self.store.delete(SLOT) {
                        Ok(_) => format!("deleted {SLOT}.json"),
                        Err(e) => format!("error: {e}"),
                    };
                }
                ui.separator();
                ui.label(egui::RichText::new(&self.save_msg).weak());
            });

        // ── Right panel: Locale ───────────────────────────────────────────────
        let score_str = self.game.score.to_string();
        let lang = self
            .langs
            .get(self.lang_idx)
            .map(|s| s.as_str())
            .unwrap_or("en");
        let start_text = self.locale.t("menu.start").into_owned();
        let quit_text = self.locale.t("menu.quit").into_owned();
        let score_text = self.locale.t_with("hud.score", &[("score", &score_str)]);
        let greet_text = self.locale.t_with("dialog.greet", &[("name", "Player")]);
        egui::Window::new("Locale")
            .default_pos(egui::pos2(720.0, 20.0))
            .default_width(220.0)
            .show(&ui_ctx, |ui| {
                ui.label(format!("Language: {lang}"));
                ui.separator();
                ui.label(&start_text);
                ui.label(&quit_text);
                ui.separator();
                ui.label(&score_text);
                ui.label(&greet_text);
                ui.separator();
                ui.label(egui::RichText::new("L  cycle language").weak());
            });

        egui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(e) = &self.egui {
            e.render(pass);
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
