use nene::input::{ActionMap, Binding, Key, MouseButton};

#[derive(Hash, PartialEq, Eq, Debug)]
enum Action {
    Jump,
    Fire,
    Dodge,
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_input() -> nene::input::Input {
    nene::input::Input::new_headless()
}

// ── ActionMap construction ────────────────────────────────────────────────────

#[test]
fn new_map_has_no_bindings() {
    let map: ActionMap<Action> = ActionMap::new();
    assert!(map.bindings(&Action::Jump).is_empty());
}

#[test]
fn bind_adds_binding() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space);
    assert_eq!(map.bindings(&Action::Jump), &[Binding::Key(Key::Space)]);
}

#[test]
fn bind_multiple_for_same_action() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space)
        .bind(Action::Jump, Key::ArrowUp);
    assert_eq!(map.bindings(&Action::Jump).len(), 2);
}

#[test]
fn bind_different_actions_independent() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space)
        .bind(Action::Fire, MouseButton::Left);
    assert_eq!(map.bindings(&Action::Jump).len(), 1);
    assert_eq!(map.bindings(&Action::Fire).len(), 1);
}

#[test]
fn rebind_replaces_existing() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space)
        .bind(Action::Jump, Key::ArrowUp);
    map.rebind(Action::Jump, Key::KeyW);
    assert_eq!(map.bindings(&Action::Jump), &[Binding::Key(Key::KeyW)]);
}

#[test]
fn unbind_clears_action() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space);
    map.unbind(&Action::Jump);
    assert!(map.bindings(&Action::Jump).is_empty());
}

#[test]
fn unbind_unknown_action_is_noop() {
    let mut map: ActionMap<Action> = ActionMap::new();
    map.unbind(&Action::Jump); // should not panic
}

// ── pressed / down / released ─────────────────────────────────────────────────

#[test]
fn unknown_action_pressed_returns_false() {
    let map: ActionMap<Action> = ActionMap::new();
    let input = make_input();
    assert!(!map.pressed(&input, &Action::Jump));
}

#[test]
fn unknown_action_down_returns_false() {
    let map: ActionMap<Action> = ActionMap::new();
    let input = make_input();
    assert!(!map.down(&input, &Action::Jump));
}

#[test]
fn unknown_action_released_returns_false() {
    let map: ActionMap<Action> = ActionMap::new();
    let input = make_input();
    assert!(!map.released(&input, &Action::Jump));
}

#[test]
fn pressed_fires_when_key_pressed() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space);

    let mut input = make_input();
    input.simulate_key_press(Key::Space);

    assert!(map.pressed(&input, &Action::Jump));
    assert!(map.down(&input, &Action::Jump));
    assert!(!map.released(&input, &Action::Jump));
}

#[test]
fn released_fires_when_key_released() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space);

    let mut input = make_input();
    input.simulate_key_press(Key::Space);
    input.begin_frame(); // advance to next frame — clears pressed, keeps held
    input.simulate_key_release(Key::Space);

    assert!(!map.pressed(&input, &Action::Jump));
    assert!(!map.down(&input, &Action::Jump));
    assert!(map.released(&input, &Action::Jump));
}

#[test]
fn either_binding_triggers_action() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space)
        .bind(Action::Jump, Key::ArrowUp);

    let mut input = make_input();
    input.simulate_key_press(Key::ArrowUp); // second binding

    assert!(map.pressed(&input, &Action::Jump));
}

#[test]
fn mouse_binding_pressed() {
    let mut map = ActionMap::new();
    map.bind(Action::Fire, MouseButton::Left);

    let mut input = make_input();
    input.simulate_mouse_press(MouseButton::Left);

    assert!(map.pressed(&input, &Action::Fire));
}

#[test]
fn unbound_action_not_triggered() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, Key::Space);

    let mut input = make_input();
    input.simulate_key_press(Key::Space);

    // Dodge has no binding — should be false even though other keys are pressed
    assert!(!map.pressed(&input, &Action::Dodge));
}

// ── Binding From impls ────────────────────────────────────────────────────────

#[test]
fn binding_from_key() {
    let b: Binding = Key::Space.into();
    assert_eq!(b, Binding::Key(Key::Space));
}

#[test]
fn binding_from_mouse() {
    let b: Binding = MouseButton::Right.into();
    assert_eq!(b, Binding::Mouse(MouseButton::Right));
}
