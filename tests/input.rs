use nene::input::{ActionMap, Binding, GamepadButton, Key, MouseButton};

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

#[test]
fn binding_from_gamepad_player() {
    let b: Binding = (1u8, GamepadButton::South).into();
    assert_eq!(b, Binding::GamepadPlayer(1, GamepadButton::South));
}

// ── GamepadPlayer bindings ─────────────────────────────────────────────────────

#[test]
fn gamepad_player_binding_pressed() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, (0u8, GamepadButton::South));

    let mut input = make_input();
    input.simulate_gamepad_press_for_player(0, GamepadButton::South);

    assert!(map.pressed(&input, &Action::Jump));
    assert!(map.down(&input, &Action::Jump));
    assert!(!map.released(&input, &Action::Jump));
}

#[test]
fn gamepad_player_binding_released() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, (0u8, GamepadButton::South));

    let mut input = make_input();
    input.simulate_gamepad_press_for_player(0, GamepadButton::South);
    input.begin_frame();
    input.simulate_gamepad_release_for_player(0, GamepadButton::South);

    assert!(!map.pressed(&input, &Action::Jump));
    assert!(!map.down(&input, &Action::Jump));
    assert!(map.released(&input, &Action::Jump));
}

#[test]
fn gamepad_player_different_players_independent() {
    let mut map_p1 = ActionMap::new();
    map_p1.bind(Action::Jump, (0u8, GamepadButton::South));

    let mut map_p2 = ActionMap::new();
    map_p2.bind(Action::Jump, (1u8, GamepadButton::South));

    let mut input = make_input();
    input.simulate_gamepad_press_for_player(0, GamepadButton::South); // only P1 presses

    assert!(map_p1.pressed(&input, &Action::Jump));
    assert!(!map_p2.pressed(&input, &Action::Jump)); // P2 unaffected
}

#[test]
fn gamepad_player_wrong_player_not_triggered() {
    let mut map = ActionMap::new();
    map.bind(Action::Jump, (1u8, GamepadButton::South)); // bound to P2

    let mut input = make_input();
    input.simulate_gamepad_press_for_player(0, GamepadButton::South); // P1 presses

    assert!(!map.pressed(&input, &Action::Jump)); // P2 binding not triggered
}

#[test]
fn gamepad_player_direct_api() {
    let mut input = make_input();
    input.simulate_gamepad_press_for_player(0, GamepadButton::East);

    assert!(input.gamepad_player_pressed(0, GamepadButton::East));
    assert!(input.gamepad_player_down(0, GamepadButton::East));
    assert!(!input.gamepad_player_released(0, GamepadButton::East));
    assert!(!input.gamepad_player_pressed(1, GamepadButton::East)); // P2 unaffected
}
