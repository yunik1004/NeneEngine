mod bindings;
pub use bindings::{ActionMap, Binding};

use std::collections::{HashMap, HashSet};

use gilrs::{EventType, GamepadId, Gilrs};
use winit::{
    event::{ElementState, MouseScrollDelta},
    keyboard::PhysicalKey,
};

use crate::math::Vec2;

pub use gilrs::{Axis as GamepadAxis, Button as GamepadButton};
pub use winit::keyboard::KeyCode as Key;

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(b: winit::event::MouseButton) -> Self {
        match b {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(n) => MouseButton::Other(n),
        }
    }
}

/// Snapshot of all input devices for the current frame.
pub struct Input {
    // Keyboard
    keys_held: HashSet<Key>,
    keys_pressed: HashSet<Key>,
    keys_released: HashSet<Key>,

    // Mouse buttons
    mouse_held: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,
    mouse_just_released: HashSet<MouseButton>,

    // Mouse movement
    mouse_pos: Vec2,
    mouse_delta: Vec2,
    scroll_delta: Vec2,

    // Gamepad (multi-pad support via GamepadId)
    gilrs: Option<Gilrs>,
    pad_held: HashSet<(GamepadId, GamepadButton)>,
    pad_pressed: HashSet<(GamepadId, GamepadButton)>,
    pad_released: HashSet<(GamepadId, GamepadButton)>,
    pad_axes: HashMap<(GamepadId, GamepadAxis), f32>,

    // Simulated per-player gamepad state (headless testing only)
    sim_player_held: HashSet<(u8, GamepadButton)>,
    sim_player_pressed: HashSet<(u8, GamepadButton)>,
    sim_player_released: HashSet<(u8, GamepadButton)>,
}

impl Input {
    pub(crate) fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_held: HashSet::new(),
            mouse_just_pressed: HashSet::new(),
            mouse_just_released: HashSet::new(),
            mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            gilrs: Gilrs::new().ok(),
            pad_held: HashSet::new(),
            pad_pressed: HashSet::new(),
            pad_released: HashSet::new(),
            pad_axes: HashMap::new(),
            sim_player_held: HashSet::new(),
            sim_player_pressed: HashSet::new(),
            sim_player_released: HashSet::new(),
        }
    }

    /// Headless [`Input`] with no window or gamepad context. Useful for tests.
    pub fn new_headless() -> Self {
        let mut s = Self::new();
        s.gilrs = None;
        s
    }

    /// Inject a key-press event (for testing or replay).
    pub fn simulate_key_press(&mut self, key: Key) {
        if !self.keys_held.contains(&key) {
            self.keys_pressed.insert(key);
            self.keys_held.insert(key);
        }
    }

    /// Inject a key-release event (for testing or replay).
    pub fn simulate_key_release(&mut self, key: Key) {
        self.keys_held.remove(&key);
        self.keys_released.insert(key);
    }

    /// Inject a mouse-button press event (for testing or replay).
    pub fn simulate_mouse_press(&mut self, button: MouseButton) {
        if !self.mouse_held.contains(&button) {
            self.mouse_just_pressed.insert(button);
            self.mouse_held.insert(button);
        }
    }

    /// Inject a mouse-button release event (for testing or replay).
    pub fn simulate_mouse_release(&mut self, button: MouseButton) {
        self.mouse_held.remove(&button);
        self.mouse_just_released.insert(button);
    }

    /// Clear per-frame transient state. Called at the start of every frame.
    /// Also useful in tests to advance to the next simulated frame.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
        self.pad_pressed.clear();
        self.pad_released.clear();
        self.sim_player_pressed.clear();
        self.sim_player_released.clear();
    }

    /// Take a snapshot of all transient (just-this-frame) state and clear it.
    ///
    /// Used by `FixedApp` so that `fixed_update` steps only see held state while
    /// the variable `update()` sees the same transient snapshot as `App::update()`.
    pub(crate) fn take_transient(&mut self) -> InputTransient {
        InputTransient {
            keys_pressed: std::mem::take(&mut self.keys_pressed),
            keys_released: std::mem::take(&mut self.keys_released),
            mouse_just_pressed: std::mem::take(&mut self.mouse_just_pressed),
            mouse_just_released: std::mem::take(&mut self.mouse_just_released),
            mouse_delta: std::mem::replace(&mut self.mouse_delta, Vec2::ZERO),
            scroll_delta: std::mem::replace(&mut self.scroll_delta, Vec2::ZERO),
            pad_pressed: std::mem::take(&mut self.pad_pressed),
            pad_released: std::mem::take(&mut self.pad_released),
        }
    }

    /// Restore transient state from a snapshot previously taken with [`take_transient`].
    pub(crate) fn restore_transient(&mut self, snap: InputTransient) {
        self.keys_pressed = snap.keys_pressed;
        self.keys_released = snap.keys_released;
        self.mouse_just_pressed = snap.mouse_just_pressed;
        self.mouse_just_released = snap.mouse_just_released;
        self.mouse_delta = snap.mouse_delta;
        self.scroll_delta = snap.scroll_delta;
        self.pad_pressed = snap.pad_pressed;
        self.pad_released = snap.pad_released;
    }

    /// Drain gilrs events and update gamepad state.
    pub(crate) fn process_gilrs(&mut self) {
        let Some(gilrs) = &mut self.gilrs else { return };
        while let Some(gilrs::Event { id, event, .. }) = gilrs.next_event() {
            match event {
                EventType::ButtonPressed(btn, _) => {
                    self.pad_held.insert((id, btn));
                    self.pad_pressed.insert((id, btn));
                }
                EventType::ButtonReleased(btn, _) => {
                    self.pad_held.remove(&(id, btn));
                    self.pad_released.insert((id, btn));
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.pad_axes.insert((id, axis), value);
                }
                EventType::Disconnected => {
                    self.pad_held.retain(|(gid, _)| *gid != id);
                    self.pad_axes.retain(|(gid, _), _| *gid != id);
                }
                _ => {}
            }
        }
    }

    pub(crate) fn on_key(&mut self, key: PhysicalKey, state: ElementState) {
        let PhysicalKey::Code(code) = key else { return };
        match state {
            ElementState::Pressed if !self.keys_held.contains(&code) => {
                self.keys_pressed.insert(code);
                self.keys_held.insert(code);
            }
            ElementState::Pressed => {}
            ElementState::Released => {
                self.keys_held.remove(&code);
                self.keys_released.insert(code);
            }
        }
    }

    pub(crate) fn on_mouse_button(
        &mut self,
        button: winit::event::MouseButton,
        state: ElementState,
    ) {
        let btn = MouseButton::from(button);
        match state {
            ElementState::Pressed if !self.mouse_held.contains(&btn) => {
                self.mouse_just_pressed.insert(btn);
                self.mouse_held.insert(btn);
            }
            ElementState::Pressed => {}
            ElementState::Released => {
                self.mouse_held.remove(&btn);
                self.mouse_just_released.insert(btn);
            }
        }
    }

    pub(crate) fn on_cursor_moved(&mut self, x: f32, y: f32) {
        self.mouse_pos = Vec2::new(x, y);
    }

    pub(crate) fn on_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.mouse_delta += Vec2::new(dx as f32, dy as f32);
    }

    pub(crate) fn on_scroll(&mut self, delta: MouseScrollDelta) {
        let (dx, dy) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0),
            MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
        };
        self.scroll_delta += Vec2::new(dx, dy);
    }

    // ── Keyboard ──────────────────────────────────────────────────────────────

    /// True every frame the key is held down.
    pub fn key_down(&self, key: Key) -> bool {
        self.keys_held.contains(&key)
    }

    /// True only on the frame the key was first pressed.
    pub fn key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// True only on the frame the key was released.
    pub fn key_released(&self, key: Key) -> bool {
        self.keys_released.contains(&key)
    }

    // ── Mouse ─────────────────────────────────────────────────────────────────

    /// True every frame the button is held.
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_held.contains(&button)
    }

    /// True only on the frame the button was first pressed.
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.contains(&button)
    }

    /// True only on the frame the button was released.
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_just_released.contains(&button)
    }

    /// Cursor position in window pixels (top-left origin).
    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    /// Raw mouse movement delta this frame (not affected by cursor acceleration).
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    /// Scroll wheel delta this frame (pixels).
    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    // ── Gamepad ───────────────────────────────────────────────────────────────

    /// True every frame the button is held on the given gamepad.
    pub fn gamepad_down(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.pad_held.contains(&(id, button))
    }

    /// True only on the frame the button was first pressed.
    pub fn gamepad_pressed(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.pad_pressed.contains(&(id, button))
    }

    /// True only on the frame the button was released.
    pub fn gamepad_released(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.pad_released.contains(&(id, button))
    }

    /// Axis value in `[-1, 1]` for the given gamepad.
    pub fn gamepad_axis(&self, id: GamepadId, axis: GamepadAxis) -> f32 {
        self.pad_axes.get(&(id, axis)).copied().unwrap_or(0.0)
    }

    /// Iterator over all currently connected gamepads.
    pub fn gamepads(&self) -> Box<dyn Iterator<Item = (GamepadId, gilrs::Gamepad<'_>)> + '_> {
        match &self.gilrs {
            Some(g) => Box::new(g.gamepads()),
            None => Box::new(std::iter::empty()),
        }
    }

    /// True every frame the button is held for the given player slot (0 = first connected pad).
    ///
    /// Falls back to simulated state when no real gamepad is connected (useful in tests).
    pub fn gamepad_player_down(&self, player: u8, btn: GamepadButton) -> bool {
        if let Some((id, _)) = self.gamepads().nth(player as usize) {
            self.gamepad_down(id, btn)
        } else {
            self.sim_player_held.contains(&(player, btn))
        }
    }

    /// True only on the frame the button was first pressed for the given player slot.
    pub fn gamepad_player_pressed(&self, player: u8, btn: GamepadButton) -> bool {
        if let Some((id, _)) = self.gamepads().nth(player as usize) {
            self.gamepad_pressed(id, btn)
        } else {
            self.sim_player_pressed.contains(&(player, btn))
        }
    }

    /// True only on the frame the button was released for the given player slot.
    pub fn gamepad_player_released(&self, player: u8, btn: GamepadButton) -> bool {
        if let Some((id, _)) = self.gamepads().nth(player as usize) {
            self.gamepad_released(id, btn)
        } else {
            self.sim_player_released.contains(&(player, btn))
        }
    }

    /// Inject a gamepad button press for a player slot (for testing or replay).
    pub fn simulate_gamepad_press_for_player(&mut self, player: u8, btn: GamepadButton) {
        if !self.sim_player_held.contains(&(player, btn)) {
            self.sim_player_pressed.insert((player, btn));
            self.sim_player_held.insert((player, btn));
        }
    }

    /// Inject a gamepad button release for a player slot (for testing or replay).
    pub fn simulate_gamepad_release_for_player(&mut self, player: u8, btn: GamepadButton) {
        self.sim_player_held.remove(&(player, btn));
        self.sim_player_released.insert((player, btn));
    }
}

// ── InputTransient ────────────────────────────────────────────────────────────

/// Snapshot of per-frame transient input state, used by `FixedApp` to restore
/// just-pressed/released events for the variable-rate update.
pub(crate) struct InputTransient {
    pub keys_pressed: HashSet<Key>,
    pub keys_released: HashSet<Key>,
    pub mouse_just_pressed: HashSet<MouseButton>,
    pub mouse_just_released: HashSet<MouseButton>,
    pub mouse_delta: Vec2,
    pub scroll_delta: Vec2,
    pub pad_pressed: HashSet<(GamepadId, GamepadButton)>,
    pub pad_released: HashSet<(GamepadId, GamepadButton)>,
}
