//! Immediate-mode UI demo.
//!
//! Shows all available widgets in two panels:
//!   • Left  — interactive controls (button, checkbox, slider)
//!   • Right — live readout of the values

use nene::{
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

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    ui: Ui,
    bg_pipeline: nene::renderer::Pipeline,

    // Widget values
    show_fps: bool,
    speed: f32,
    intensity: f32,
    brightness: f32,
    click_count: u32,
    last_message: String,
}

const W: f32 = 900.0;
const H: f32 = 560.0;

fn init(ctx: &mut Context) -> State {
    let bg_pipeline = ctx.create_pipeline(nene::renderer::PipelineDescriptor::fullscreen_pass(
        BG_SHADER,
    ));

    State {
        ui: Ui::new(ctx),
        bg_pipeline,
        show_fps: true,
        speed: 3.5,
        intensity: 0.7,
        brightness: 1.0,
        click_count: 0,
        last_message: "–".to_string(),
    }
}

fn main() {
    Window::new(Config {
        title: "UI demo".to_string(),
        width: W as u32,
        height: H as u32,
        ..Config::default()
    })
    .run_with_update(
        init,
        // ── update ──────────────────────────────────────────────────────────
        |state, ctx, input, time| {
            state.ui.begin_frame(input, W, H);

            // ── Left panel: controls ─────────────────────────────────────────
            state.ui.begin_panel("Controls", 20.0, 20.0, 220.0);

            state.ui.label("Simulation");
            state.ui.separator();

            if state.ui.button("Fire Burst") {
                state.click_count += 1;
                state.last_message = format!("burst #{}", state.click_count);
            }
            if state.ui.button("Reset Values") {
                state.speed = 3.5;
                state.intensity = 0.7;
                state.brightness = 1.0;
                state.last_message = "values reset".to_string();
            }

            state.ui.separator();
            state.ui.label("Display");
            state.ui.separator();

            state.ui.checkbox("Show FPS", &mut state.show_fps);

            state.ui.separator();
            state.ui.label("Parameters");
            state.ui.separator();

            state.ui.slider("Speed", &mut state.speed, 0.0, 10.0);
            state.ui.slider("Intensity", &mut state.intensity, 0.0, 1.0);
            state
                .ui
                .slider("Brightness", &mut state.brightness, 0.0, 2.0);

            state.ui.end_panel();

            // ── Right panel: readout ─────────────────────────────────────────
            state.ui.begin_panel("Readout", 260.0, 20.0, 200.0);

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

            let fps_text = if state.show_fps {
                let fps = if time.delta > 0.0 {
                    (1.0 / time.delta) as u32
                } else {
                    0
                };
                format!("FPS  {fps}")
            } else {
                "FPS  —".to_string()
            };
            state.ui.label(&fps_text);

            state.ui.separator();
            state.ui.label("Last action");
            state.ui.label_dim(&state.last_message.clone());

            state.ui.end_panel();

            state.ui.end_frame(ctx);
        },
        |_, _| {},
        // ── render ──────────────────────────────────────────────────────────
        |state, pass: &mut RenderPass| {
            // Dark background
            pass.set_pipeline(&state.bg_pipeline);
            pass.draw(0..3);

            // UI on top
            state.ui.render(pass);
        },
    );
}
