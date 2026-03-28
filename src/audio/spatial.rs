use std::sync::Arc;

use crate::math::Vec2;

use super::device::{AudioDevice, PlayHandle, PlayOptions};
use super::sound::Sound;

/// Listener configuration for spatial (positional) audio.
///
/// Derive stereo pan and distance attenuation from 2-D world coordinates.
pub struct SpatialAudio {
    /// World position of the listener.
    pub listener: Vec2,
    /// Distance at which sounds become inaudible.
    pub max_distance: f32,
}

impl SpatialAudio {
    pub fn new(max_distance: f32) -> Self {
        Self {
            listener: Vec2::ZERO,
            max_distance: max_distance.max(f32::EPSILON),
        }
    }

    pub fn options_for(&self, emitter_pos: Vec2) -> PlayOptions {
        let (volume, pan) = self.compute(emitter_pos);
        PlayOptions {
            volume,
            pan,
            looping: false,
        }
    }

    pub fn play(&self, device: &AudioDevice, sound: &Arc<Sound>, emitter_pos: Vec2) -> PlayHandle {
        device.play_with(sound, self.options_for(emitter_pos))
    }

    pub fn play_source(
        &self,
        device: &AudioDevice,
        sound: &Arc<Sound>,
        emitter_pos: Vec2,
        looping: bool,
    ) -> SpatialSource {
        let opts = PlayOptions {
            looping,
            ..self.options_for(emitter_pos)
        };
        let handle = device.play_with(sound, opts);
        SpatialSource {
            handle,
            pos: emitter_pos,
        }
    }

    pub(super) fn compute(&self, emitter_pos: Vec2) -> (f32, f32) {
        let dx = emitter_pos.x - self.listener.x;
        let dy = emitter_pos.y - self.listener.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let volume = (1.0 - dist / self.max_distance).clamp(0.0, 1.0);
        let pan = (dx / self.max_distance).clamp(-1.0, 1.0);
        (volume, pan)
    }
}

/// A playing sound whose spatial parameters update as the emitter moves.
pub struct SpatialSource {
    handle: PlayHandle,
    pos: Vec2,
}

impl SpatialSource {
    pub fn set_position(&mut self, spatial: &SpatialAudio, pos: Vec2) {
        self.pos = pos;
        let (volume, pan) = spatial.compute(pos);
        self.handle.set_volume(volume);
        self.handle.set_pan(pan);
    }

    pub fn position(&self) -> Vec2 {
        self.pos
    }
    pub fn stop(&self) {
        self.handle.stop();
    }
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}
