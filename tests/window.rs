use nene::window::{Config, Window};

#[test]
fn default_config() {
    let config = Config::default();
    assert_eq!(config.title, "Nene");
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
}

#[test]
fn custom_config() {
    let config = Config {
        title: "Test".to_string(),
        width: 800,
        height: 600,
    };
    assert_eq!(config.title, "Test");
    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
}

#[test]
fn partial_default_config() {
    let config = Config {
        title: "MyGame".to_string(),
        ..Config::default()
    };
    assert_eq!(config.title, "MyGame");
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
}

#[test]
fn window_new() {
    let _window = Window::new(Config::default());
}

#[test]
fn window_new_custom() {
    let _window = Window::new(Config {
        title: "Custom".to_string(),
        width: 1920,
        height: 1080,
    });
}
