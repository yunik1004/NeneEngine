use nene::window::{Config, Window};

fn main() {
    let window = Window::new(Config {
        title: "Nene - Window Test".to_string(),
        width: 1280,
        height: 720,
    });

    window.run();
}
