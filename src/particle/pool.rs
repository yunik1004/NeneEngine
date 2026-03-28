use super::emitter::{EmitterConfig, ParticleInstance};

struct Particle {
    pos: [f32; 3],
    vel: [f32; 3],
    life: f32,
    max_life: f32,
    size_start: f32,
    size_end: f32,
    color_start: [f32; 4],
    color_end: [f32; 4],
}

/// CPU-side particle simulation. No GPU dependencies — fully unit-testable.
pub struct ParticlePool {
    pub config: EmitterConfig,
    particles: Vec<Particle>,
    emit_accum: f32,
    rng: u32,
}

impl ParticlePool {
    pub fn new(config: EmitterConfig) -> Self {
        Self {
            particles: Vec::with_capacity(config.max_particles),
            emit_accum: 0.0,
            rng: 0xDEAD_BEEF,
            config,
        }
    }

    // xorshift32 PRNG — no external dependency
    fn rand(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        // map to [0, 1)
        (x as f32) / (u32::MAX as f32)
    }

    fn rand_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.rand() * (hi - lo)
    }

    fn rand_unit_sphere(&mut self) -> [f32; 3] {
        loop {
            let x = self.rand_range(-1.0, 1.0);
            let y = self.rand_range(-1.0, 1.0);
            let z = self.rand_range(-1.0, 1.0);
            let len = (x * x + y * y + z * z).sqrt();
            if len > 0.001 && len <= 1.0 {
                return [x / len, y / len, z / len];
            }
        }
    }

    fn spawn_one(&mut self, pos: [f32; 3]) {
        if self.particles.len() >= self.config.max_particles {
            return;
        }
        // Copy config values before calling mutable rand methods
        let base_life = self.config.lifetime;
        let life_var = self.config.lifetime_variance;
        let base_speed = self.config.speed;
        let speed_var = self.config.speed_variance;
        let spread_raw = self.config.spread;
        let dir = self.config.direction;
        let size_start = self.config.size_start;
        let size_end = self.config.size_end;
        let color_start = self.config.color_start;
        let color_end = self.config.color_end;

        let life = (base_life + self.rand_range(-life_var, life_var)).max(0.05);
        let speed = (base_speed + self.rand_range(-speed_var, speed_var)).max(0.0);

        // Blend direction with random sphere point, weighted by spread
        let spread = spread_raw.clamp(0.0, std::f32::consts::PI);
        let rnd = self.rand_unit_sphere();
        // lerp between pure direction and random
        let t = spread / std::f32::consts::PI;
        let vx = dir[0] * (1.0 - t) + rnd[0] * t;
        let vy = dir[1] * (1.0 - t) + rnd[1] * t;
        let vz = dir[2] * (1.0 - t) + rnd[2] * t;
        let len = (vx * vx + vy * vy + vz * vz).sqrt().max(1e-6);

        self.particles.push(Particle {
            pos,
            vel: [vx / len * speed, vy / len * speed, vz / len * speed],
            life,
            max_life: life,
            size_start,
            size_end,
            color_start,
            color_end,
        });
    }

    /// Advance simulation by `dt` seconds, emitting from `emitter_pos`.
    pub fn update(&mut self, dt: f32, emitter_pos: [f32; 3]) {
        // Simulate existing particles
        let g = self.config.gravity;
        self.particles.retain_mut(|p| {
            p.life -= dt;
            if p.life <= 0.0 {
                return false;
            }
            p.vel[1] += g * dt;
            p.pos[0] += p.vel[0] * dt;
            p.pos[1] += p.vel[1] * dt;
            p.pos[2] += p.vel[2] * dt;
            true
        });

        // Continuous emission
        if self.config.emit_rate > 0.0 {
            self.emit_accum += self.config.emit_rate * dt;
            let count = self.emit_accum as usize;
            self.emit_accum -= count as f32;
            for _ in 0..count {
                self.spawn_one(emitter_pos);
            }
        }
    }

    /// Instantly spawn `count` particles from `pos`.
    pub fn burst(&mut self, count: usize, pos: [f32; 3]) {
        for _ in 0..count {
            self.spawn_one(pos);
        }
    }

    /// Remove all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Number of currently live particles.
    pub fn active_count(&self) -> usize {
        self.particles.len()
    }

    /// Build the per-instance GPU data for all live particles.
    ///
    /// `cam_right` and `cam_up` are the camera's right/up axes in world space,
    /// extracted from the view matrix rows.
    pub fn instances(&self) -> Vec<ParticleInstance> {
        self.particles
            .iter()
            .map(|p| {
                let t = 1.0 - p.life / p.max_life;
                let size = lerp_f32(p.size_start, p.size_end, t);
                let color = lerp_rgba(p.color_start, p.color_end, t);
                ParticleInstance {
                    pos_size: [p.pos[0], p.pos[1], p.pos[2], size],
                    color,
                }
            })
            .collect()
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_rgba(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp_f32(a[0], b[0], t),
        lerp_f32(a[1], b[1], t),
        lerp_f32(a[2], b[2], t),
        lerp_f32(a[3], b[3], t),
    ]
}
