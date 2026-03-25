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
    gilrs: Gilrs,
    pad_held: HashSet<(GamepadId, GamepadButton)>,
    pad_pressed: HashSet<(GamepadId, GamepadButton)>,
    pad_released: HashSet<(GamepadId, GamepadButton)>,
    pad_axes: HashMap<(GamepadId, GamepadAxis), f32>,
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
            gilrs: Gilrs::new().expect("gilrs init failed"),
            pad_held: HashSet::new(),
            pad_pressed: HashSet::new(),
            pad_released: HashSet::new(),
            pad_axes: HashMap::new(),
        }
    }

    /// Clear per-frame state. Called at the start of every frame.
    pub(crate) fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
        self.pad_pressed.clear();
        self.pad_released.clear();
    }

    /// Drain gilrs events and update gamepad state.
    pub(crate) fn process_gilrs(&mut self) {
        while let Some(gilrs::Event { id, event, .. }) = self.gilrs.next_event() {
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
    pub fn gamepads(&self) -> impl Iterator<Item = (GamepadId, gilrs::Gamepad<'_>)> {
        self.gilrs.gamepads()
    }
}
