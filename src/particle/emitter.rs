/// A 2-D corner offset for the billboard quad (`[-0.5, -0.5]` … `[0.5, 0.5]`).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct QuadVert {
    pub corner: [f32; 2],
}

/// Per-instance data uploaded to the GPU each frame.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleInstance {
    /// `xyz` = world position, `w` = half-size (radius).
    pub pos_size: [f32; 4],
    /// RGBA colour (alpha drives overall opacity).
    pub color: [f32; 4],
}

/// All tunable parameters for a [`ParticlePool`](super::pool::ParticlePool).
#[derive(Clone, Debug)]
pub struct EmitterConfig {
    /// Maximum live particles at any time.
    pub max_particles: usize,
    /// New particles spawned per second (continuous emission).
    pub emit_rate: f32,
    /// Base particle lifetime in seconds.
    pub lifetime: f32,
    /// ± variance added to `lifetime`.
    pub lifetime_variance: f32,
    /// Base launch speed (world units / second).
    pub speed: f32,
    /// ± variance added to `speed`.
    pub speed_variance: f32,
    /// Particle size (world units) at birth.
    pub size_start: f32,
    /// Particle size at death (linearly interpolated).
    pub size_end: f32,
    /// RGBA colour at birth.
    pub color_start: [f32; 4],
    /// RGBA colour at death (lerped).
    pub color_end: [f32; 4],
    /// Downward gravitational pull (world units / s²).
    pub gravity: f32,
    /// Launch direction (normalised).
    pub direction: [f32; 3],
    /// Half-angle of the emission cone in radians.
    pub spread: f32,
}

impl EmitterConfig {
    /// Upward fire: orange→red, additive, fast emit.
    pub fn fire() -> Self {
        Self {
            max_particles: 512,
            emit_rate: 80.0,
            lifetime: 1.2,
            lifetime_variance: 0.4,
            speed: 3.0,
            speed_variance: 1.5,
            size_start: 0.35,
            size_end: 0.05,
            color_start: [1.0, 0.55, 0.05, 1.0],
            color_end: [0.8, 0.1, 0.0, 0.0],
            gravity: -1.5,
            direction: [0.0, 1.0, 0.0],
            spread: 0.4,
        }
    }

    /// Burst of sparks in all directions.
    pub fn sparks() -> Self {
        Self {
            max_particles: 256,
            emit_rate: 0.0, // burst-only
            lifetime: 0.8,
            lifetime_variance: 0.3,
            speed: 6.0,
            speed_variance: 2.0,
            size_start: 0.15,
            size_end: 0.0,
            color_start: [1.0, 0.9, 0.3, 1.0],
            color_end: [1.0, 0.3, 0.0, 0.0],
            gravity: -8.0,
            direction: [0.0, 1.0, 0.0],
            spread: std::f32::consts::PI,
        }
    }
}
