//! Spatial (positional) audio — derive stereo pan and distance attenuation
//! from 2-D world coordinates.
//!
//! # Model
//!
//! * **Attenuation** — linear falloff: `volume = (1 − dist / max_distance).max(0)`
//! * **Pan** — horizontal position relative to the listener:
//!   `pan = (emitter.x − listener.x) / max_distance`, clamped to `[−1, 1]`
//!
//! This matches the expectations of most 2-D games.  For 3-D or HRTF-quality
//! spatialization a dedicated audio middleware (e.g. FMOD, Wwise) is recommended.
//!
//! # Example
//! ```no_run
//! use std::sync::Arc;
//! use nene::audio::{AudioDevice, SpatialAudio};
//! use nene::math::Vec2;
//!
//! let audio = AudioDevice::new();
//! let sound: Arc<_> = todo!();
//!
//! let mut spatial = SpatialAudio::new(20.0); // audible within 20 world units
//! spatial.listener = Vec2::new(0.0, 0.0);
//!
//! // One-shot sound at a fixed position.
//! let _handle = spatial.play(&audio, &sound, Vec2::new(5.0, 0.0));
//!
//! // Moving source — update its position every frame.
//! let mut src = spatial.play_source(&audio, &sound, Vec2::ZERO, true);
//! // … game loop …
//! src.set_position(&spatial, Vec2::new(3.0, 0.0));
//! ```

use std::sync::Arc;

use super::{AudioDevice, PlayHandle, PlayOptions, Sound};
use crate::math::Vec2;

// ── SpatialAudio ──────────────────────────────────────────────────────────────

/// Listener configuration for spatial audio.
///
/// Keep one instance per scene; update [`listener`](Self::listener) each frame
/// to follow the camera or player.
pub struct SpatialAudio {
    /// World position of the listener (typically the camera or player).
    pub listener: Vec2,
    /// Distance at which sounds are completely inaudible.  Sounds within
    /// this radius are attenuated linearly from full volume (distance 0) to
    /// silence (distance = `max_distance`).
    pub max_distance: f32,
}

impl SpatialAudio {
    /// Create a new `SpatialAudio` with the listener at the origin.
    pub fn new(max_distance: f32) -> Self {
        Self {
            listener: Vec2::ZERO,
            max_distance: max_distance.max(f32::EPSILON),
        }
    }

    /// Compute [`PlayOptions`] for a sound at `emitter_pos`.
    pub fn options_for(&self, emitter_pos: Vec2) -> PlayOptions {
        let (volume, pan) = self.compute(emitter_pos);
        PlayOptions {
            volume,
            pan,
            looping: false,
        }
    }

    /// Play a one-shot sound at `emitter_pos`.
    ///
    /// The pan and volume are computed once at the time of the call and do
    /// not update if the emitter or listener moves.  For moving sources use
    /// [`play_source`](Self::play_source).
    pub fn play(&self, device: &AudioDevice, sound: &Arc<Sound>, emitter_pos: Vec2) -> PlayHandle {
        device.play_with(sound, self.options_for(emitter_pos))
    }

    /// Play a sound at `emitter_pos` and return a [`SpatialSource`] whose
    /// pan and volume can be updated every frame as the emitter moves.
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

    // ── internal ──────────────────────────────────────────────────────────────

    /// Returns `(volume, pan)` for a given emitter position.
    fn compute(&self, emitter_pos: Vec2) -> (f32, f32) {
        let dx = emitter_pos.x - self.listener.x;
        let dy = emitter_pos.y - self.listener.y;
        let dist = (dx * dx + dy * dy).sqrt();

        let volume = (1.0 - dist / self.max_distance).clamp(0.0, 1.0);
        let pan = (dx / self.max_distance).clamp(-1.0, 1.0);
        (volume, pan)
    }
}

// ── SpatialSource ─────────────────────────────────────────────────────────────

/// A playing sound whose spatial parameters update as the emitter moves.
///
/// Obtain from [`SpatialAudio::play_source`].  Call [`set_position`](Self::set_position)
/// each frame to keep pan and volume in sync with the emitter's world position.
pub struct SpatialSource {
    handle: PlayHandle,
    pos: Vec2,
}

impl SpatialSource {
    /// Update the emitter's world position, recomputing pan and volume.
    ///
    /// Call this once per frame (or whenever the source moves).
    pub fn set_position(&mut self, spatial: &SpatialAudio, pos: Vec2) {
        self.pos = pos;
        let (volume, pan) = spatial.compute(pos);
        self.handle.set_volume(volume);
        self.handle.set_pan(pan);
    }

    /// Current emitter position.
    pub fn position(&self) -> Vec2 {
        self.pos
    }

    /// Stop playback early.
    pub fn stop(&self) {
        self.handle.stop();
    }

    /// `true` when the sound has finished or been stopped.
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}
