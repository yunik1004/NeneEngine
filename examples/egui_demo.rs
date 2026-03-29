/// egui integration demo — a floating panel with controls.
use nene::{
    app::{App, Config, WindowEvent, WindowId, run},
    input::Input,
    renderer::{Context, RenderPass},
    time::Time,
    ui::EguiUi,
};

struct EguiDemo {
    egui: Option<EguiUi>,
    name: String,
    age: f32,
    dark: bool,
}

impl App for EguiDemo {
    fn new() -> Self {
        Self {
            egui: None,
            name: "nene".to_owned(),
            age: 25.0,
            dark: true,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let e = EguiUi::new(ctx);
        // Dark theme by default — flip to light if you prefer:
        // egui::Context::set_visuals(&egui::Visuals::light());
        self.egui = Some(e);
    }

    fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
        if let Some(e) = &mut self.egui {
            e.handle_event(event);
        }
    }

    fn update(&mut self, _input: &Input, _time: &Time) {}

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let Some(egui) = &mut self.egui else { return };

        let ui_ctx = egui.begin_frame();

        egui::Window::new("Demo")
            .resizable(true)
            .default_width(280.0)
            .show(&ui_ctx, |ui| {
                ui.heading("egui + nene");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.name);
                });

                ui.add(egui::Slider::new(&mut self.age, 0.0..=120.0).text("Age"));

                if ui.button(format!("Say hello to {}", self.name)).clicked() {
                    println!("Hello, {}! You are {:.0} years old.", self.name, self.age);
                }

                ui.separator();

                ui.checkbox(&mut self.dark, "Dark mode");
                if self.dark {
                    ui_ctx.set_visuals(egui::Visuals::dark());
                } else {
                    ui_ctx.set_visuals(egui::Visuals::light());
                }

                ui.separator();
                ui.label(
                    egui::RichText::new("Rich text")
                        .color(egui::Color32::GOLD)
                        .size(20.0),
                );
                ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "Colored label");
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
            title: "egui demo",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<EguiDemo>();
}
