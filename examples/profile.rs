//! Profiler overlay demo.
//!
//! Shows frame time, FPS, and named scope timings in a UI panel.
//! Intentionally adds a small artificial load in the "work" scope
//! so the timings are non-trivial.

use nene::{
    profile::Profiler,
    renderer::{Context, RenderPass},
    ui::Ui,
    window::{Config, Window},
};
use std::hint::black_box;

const W: u32 = 480;
const H: u32 = 360;

struct State {
    profiler: Profiler,
    ui: Ui,
}

fn init(ctx: &mut Context) -> State {
    State {
        profiler: Profiler::new(),
        ui: Ui::new(ctx),
    }
}

fn main() {
    Window::new(Config {
        title: "Profiler demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, _time| {
            state.profiler.begin_frame();

            {
                let _s = state.profiler.scope("update");
                // Simulate some update work.
                let mut x = 0.0f64;
                for i in 0..50_000 {
                    x = black_box(x + (i as f64).sin());
                }
                let _ = black_box(x);
            }

            // Build UI — overlay shows stats from the previous frame.
            state.ui.begin_frame(input, W as f32, H as f32);
            state.profiler.draw_overlay(&mut state.ui, 20.0, 20.0);
            state.ui.end_frame(ctx);

            state.profiler.end_frame();
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.ui.render(pass);
        },
    );
}
