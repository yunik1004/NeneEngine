//! CPU particle simulation + GPU billboard rendering.
//!
//! # Quick start
//! ```no_run
//! use nene::particle::{EmitterConfig, ParticleSystem};
//! use nene::math::Vec3;
//!
//! // In init:
//! // let mut fire = ParticleSystem::new(&ctx, EmitterConfig::fire());
//!
//! // In update (per frame):
//! // let view_proj = camera.view_proj(aspect);
//! // let cam_right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
//! // let cam_up    = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
//! // fire.update(time.delta, emitter_pos, view_proj, cam_right, cam_up, &ctx);
//!
//! // In render:
//! // fire.draw(&mut pass);
//! ```

use crate::math::{Mat4, Vec3};
use crate::renderer::{
    Context, InstanceBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    VertexFormat, VertexLayout,
};

// ── WGSL shader ───────────────────────────────────────────────────────────────

const SHADER: &str = r#"
struct Uniforms {
    view_proj : mat4x4<f32>,
    cam_right : vec3<f32>,
    cam_up    : vec3<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertOut {
    @builtin(position) clip  : vec4<f32>,
    @location(0)       color : vec4<f32>,
    @location(1)       uv    : vec2<f32>,
}

@vertex fn vs_main(
    @location(0) corner  : vec2<f32>,
    @location(1) pos_size: vec4<f32>,
    @location(2) color   : vec4<f32>,
) -> VertOut {
    let world = pos_size.xyz
        + u.cam_right * corner.x * pos_size.w
        + u.cam_up   * corner.y * pos_size.w;
    var out: VertOut;
    out.clip  = u.view_proj * vec4<f32>(world, 1.0);
    out.color = color;
    out.uv    = corner + vec2<f32>(0.5);
    return out;
}

@fragment fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    // Soft circle — fade to transparent at edge
    let d = length(in.uv - vec2<f32>(0.5)) * 2.0;
    let alpha = clamp(1.0 - d * d, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

// ── GPU vertex / instance types ───────────────────────────────────────────────

/// A 2-D corner offset for the billboard quad (`[-0.5, -0.5]` … `[0.5, 0.5]`).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVert {
    corner: [f32; 2],
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

// ── EmitterConfig ─────────────────────────────────────────────────────────────

/// All tunable parameters for a [`ParticlePool`].
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

// ── CPU particle pool ─────────────────────────────────────────────────────────

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

// ── GPU uniform ───────────────────────────────────────────────────────────────

#[derive(encase::ShaderType)]
struct ParticleUniform {
    view_proj: Mat4,
    cam_right: Vec3,
    cam_up: Vec3,
}

pub const MAX_PARTICLES: usize = 4096;

// ── ParticleSystem (GPU) ──────────────────────────────────────────────────────

/// GPU-backed particle system. Wraps [`ParticlePool`] with a pipeline, quad
/// vertex buffer, and pre-allocated instance buffer.
pub struct ParticleSystem {
    pool: ParticlePool,
    pipeline: Pipeline,
    /// Unit-quad vertex buffer (6 vertices, no index buffer).
    quad_vbuf: VertexBuffer,
    inst_buf: InstanceBuffer,
    ubuf: UniformBuffer,
    inst_count: u32,
}

impl ParticleSystem {
    pub fn new(ctx: &Context, config: EmitterConfig) -> Self {
        // Vertex layout: slot 0 = corner (Float32x2)
        let vert_layout = VertexLayout {
            stride: std::mem::size_of::<QuadVert>() as u64,
            attributes: vec![crate::renderer::VertexAttribute {
                offset: 0,
                location: 0,
                format: VertexFormat::Float32x2,
            }],
        };
        // Instance layout: slot 1 = pos_size (loc 1, Float32x4), color (loc 2, Float32x4)
        let inst_layout = VertexLayout {
            stride: std::mem::size_of::<ParticleInstance>() as u64,
            attributes: vec![
                crate::renderer::VertexAttribute {
                    offset: 0,
                    location: 1,
                    format: VertexFormat::Float32x4,
                },
                crate::renderer::VertexAttribute {
                    offset: 16,
                    location: 2,
                    format: VertexFormat::Float32x4,
                },
            ],
        };

        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(SHADER, vert_layout)
                .with_instance_layout(inst_layout)
                .with_uniform()
                .with_additive_blend(),
        );

        // 6-vertex unit quad (two triangles)
        let quad_vbuf = ctx.create_vertex_buffer(&[
            QuadVert {
                corner: [-0.5, -0.5],
            },
            QuadVert {
                corner: [0.5, -0.5],
            },
            QuadVert { corner: [0.5, 0.5] },
            QuadVert {
                corner: [-0.5, -0.5],
            },
            QuadVert { corner: [0.5, 0.5] },
            QuadVert {
                corner: [-0.5, 0.5],
            },
        ]);

        // Pre-allocate instance buffer for MAX_PARTICLES
        let dummy = vec![
            ParticleInstance {
                pos_size: [0.0; 4],
                color: [0.0; 4]
            };
            MAX_PARTICLES
        ];
        let inst_buf = ctx.create_instance_buffer(&dummy);

        let ubuf = ctx.create_uniform_buffer(&ParticleUniform {
            view_proj: Mat4::IDENTITY,
            cam_right: Vec3::X,
            cam_up: Vec3::Y,
        });

        Self {
            pool: ParticlePool::new(config),
            pipeline,
            quad_vbuf,
            inst_buf,
            ubuf,
            inst_count: 0,
        }
    }

    /// Borrow the underlying CPU pool (e.g. to call `burst`).
    pub fn pool(&self) -> &ParticlePool {
        &self.pool
    }

    pub fn pool_mut(&mut self) -> &mut ParticlePool {
        &mut self.pool
    }

    /// Advance simulation and upload instance data to the GPU.
    ///
    /// `cam_right` / `cam_up` come from the view matrix rows — see example.
    pub fn update(
        &mut self,
        dt: f32,
        emitter_pos: Vec3,
        view_proj: Mat4,
        cam_right: Vec3,
        cam_up: Vec3,
        ctx: &Context,
    ) {
        self.pool.update(dt, emitter_pos.to_array());

        ctx.update_uniform_buffer(
            &self.ubuf,
            &ParticleUniform {
                view_proj,
                cam_right,
                cam_up,
            },
        );

        let instances = self.pool.instances();
        let count = instances.len().min(MAX_PARTICLES);
        if count > 0 {
            ctx.update_instance_buffer(&self.inst_buf, &instances[..count]);
        }
        self.inst_count = count as u32;
    }

    /// Instantly spawn `count` particles from `pos`.
    pub fn burst(&mut self, count: usize, pos: Vec3) {
        self.pool.burst(count, pos.to_array());
    }

    /// Bind pipeline and draw all live particles.
    pub fn draw(&self, pass: &mut RenderPass) {
        if self.inst_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_vertex_buffer(0, &self.quad_vbuf);
        pass.set_instance_buffer(1, &self.inst_buf);
        pass.draw_instanced(0..6, self.inst_count);
    }
}
