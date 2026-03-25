use nene::{
    renderer::RenderPass,
    text::TextRenderer,
    window::{Config, Window},
};

struct State {
    text: TextRenderer,
    frame: u32,
}

fn main() {
    Window::new(Config {
        title: "Text".to_string(),
        ..Config::default()
    })
    .run_with_update(
        |ctx| State {
            text: TextRenderer::new(ctx),
            frame: 0,
        },
        |state, ctx| {
            state.frame += 1;
            state.text.clear();
            state
                .text
                .queue("Hello, Nene!", 50.0, 80.0, 48.0, [1.0, 1.0, 1.0, 1.0]);
            state.text.queue(
                &format!("Frame: {}", state.frame),
                50.0,
                140.0,
                28.0,
                [0.8, 0.8, 0.2, 1.0],
            );
            state.text.queue(
                "cosmic-text + wgpu",
                50.0,
                190.0,
                22.0,
                [0.4, 0.9, 0.6, 1.0],
            );
            state.text.prepare(ctx);
        },
        |state, pass: &mut RenderPass| {
            pass.draw_text(&state.text);
        },
    );
}
