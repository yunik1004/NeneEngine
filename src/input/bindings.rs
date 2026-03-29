use std::collections::HashMap;
use std::hash::Hash;

use super::{GamepadButton, Input, Key, MouseButton};

/// A single physical input that can trigger an action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Binding {
    Key(Key),
    Mouse(MouseButton),
    /// Matches the button on any connected gamepad.
    Gamepad(GamepadButton),
    /// Matches the button on the gamepad assigned to a specific player slot (0-based).
    /// Player 0 = first connected gamepad, player 1 = second, etc.
    GamepadPlayer(u8, GamepadButton),
}

impl From<Key> for Binding {
    fn from(k: Key) -> Self {
        Binding::Key(k)
    }
}

impl From<MouseButton> for Binding {
    fn from(b: MouseButton) -> Self {
        Binding::Mouse(b)
    }
}

impl From<GamepadButton> for Binding {
    fn from(b: GamepadButton) -> Self {
        Binding::Gamepad(b)
    }
}

impl From<(u8, GamepadButton)> for Binding {
    fn from((player, btn): (u8, GamepadButton)) -> Self {
        Binding::GamepadPlayer(player, btn)
    }
}

/// Maps named actions to one or more physical [`Binding`]s.
///
/// The action type `A` is typically a user-defined enum:
///
/// ```
/// # use nene::input::{ActionMap, Key, MouseButton};
/// #[derive(Hash, PartialEq, Eq)]
/// enum Action { Jump, Fire }
///
/// let mut map = ActionMap::new();
/// map.bind(Action::Jump, Key::Space)
///    .bind(Action::Fire, MouseButton::Left);
/// ```
///
/// Query in `update()`:
/// ```ignore
/// if map.pressed(input, &Action::Jump) { /* … */ }
/// ```
pub struct ActionMap<A: Hash + Eq> {
    bindings: HashMap<A, Vec<Binding>>,
}

impl<A: Hash + Eq> Default for ActionMap<A> {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

impl<A: Hash + Eq> ActionMap<A> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding for an action. Multiple bindings per action are allowed;
    /// any one of them firing is enough to trigger the action.
    pub fn bind(&mut self, action: A, binding: impl Into<Binding>) -> &mut Self {
        self.bindings
            .entry(action)
            .or_default()
            .push(binding.into());
        self
    }

    /// Replace all bindings for an action with a single new one.
    pub fn rebind(&mut self, action: A, binding: impl Into<Binding>) -> &mut Self {
        self.bindings.insert(action, vec![binding.into()]);
        self
    }

    /// Remove all bindings for an action.
    pub fn unbind(&mut self, action: &A) {
        self.bindings.remove(action);
    }

    /// All bindings currently registered for `action`.
    pub fn bindings(&self, action: &A) -> &[Binding] {
        self.bindings.get(action).map(Vec::as_slice).unwrap_or(&[])
    }

    /// True only on the frame the action was first triggered.
    pub fn pressed(&self, input: &Input, action: &A) -> bool {
        self.any(action, |b| binding_pressed(input, b))
    }

    /// True every frame at least one binding for the action is held.
    pub fn down(&self, input: &Input, action: &A) -> bool {
        self.any(action, |b| binding_down(input, b))
    }

    /// True only on the frame all held bindings for the action were released.
    pub fn released(&self, input: &Input, action: &A) -> bool {
        self.any(action, |b| binding_released(input, b))
    }

    fn any(&self, action: &A, f: impl Fn(&Binding) -> bool) -> bool {
        self.bindings.get(action).is_some_and(|bs| bs.iter().any(f))
    }
}

fn binding_pressed(input: &Input, b: &Binding) -> bool {
    match b {
        Binding::Key(k) => input.key_pressed(*k),
        Binding::Mouse(m) => input.mouse_pressed(*m),
        Binding::Gamepad(btn) => input
            .gamepads()
            .any(|(id, _)| input.gamepad_pressed(id, *btn)),
        Binding::GamepadPlayer(player, btn) => input.gamepad_player_pressed(*player, *btn),
    }
}

fn binding_down(input: &Input, b: &Binding) -> bool {
    match b {
        Binding::Key(k) => input.key_down(*k),
        Binding::Mouse(m) => input.mouse_down(*m),
        Binding::Gamepad(btn) => input.gamepads().any(|(id, _)| input.gamepad_down(id, *btn)),
        Binding::GamepadPlayer(player, btn) => input.gamepad_player_down(*player, *btn),
    }
}

fn binding_released(input: &Input, b: &Binding) -> bool {
    match b {
        Binding::Key(k) => input.key_released(*k),
        Binding::Mouse(m) => input.mouse_released(*m),
        Binding::Gamepad(btn) => input
            .gamepads()
            .any(|(id, _)| input.gamepad_released(id, *btn)),
        Binding::GamepadPlayer(player, btn) => input.gamepad_player_released(*player, *btn),
    }
}
